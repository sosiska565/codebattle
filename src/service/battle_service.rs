use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::future::join_all;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::battle_room::{
    BattleManager, BattleRoom, DAMAGE_PER_SOLVE, Finish, Player, PlayerInfo, START_HP, ServerMsg,
    TaskState,
};
use crate::models::dto::battle_dto::BattleHistoryItem;
use crate::repository::battle_repository::BattleRepository;
use crate::repository::user_repository::UserRepository;
use crate::service::piston_service::{ExecOutcome, PistonService, piston_language};
use crate::service::problem_service::{ProblemEvent, ProblemService, normalize_output};
use crate::service::redis_service::RedisService;

const LEVEL_MIN_ELO: [i32; 10] = [0, 801, 951, 1101, 1251, 1401, 1551, 1701, 1851, 2001];
const ELO_K: f64 = 32.0;

const MAX_GENERATION_ATTEMPTS: u32 = 2;
const DISCONNECT_GRACE: Duration = Duration::from_secs(30);
const JOIN_TIMEOUT: Duration = Duration::from_secs(90);
const FINISHED_ROOM_KEEP: Duration = Duration::from_secs(10 * 60);
const MAX_CODE_BYTES: usize = 64 * 1024;
const MAX_MESSAGE_CHARS: usize = 1500;

fn level_for_elo(elo: i32) -> i32 {
    LEVEL_MIN_ELO
        .iter()
        .rposition(|min| elo >= *min)
        .map(|i| i as i32 + 1)
        .unwrap_or(1)
}

fn elo_change(winner: i32, loser: i32) -> i32 {
    let expected = 1.0 / (1.0 + 10f64.powf((loser - winner) as f64 / 400.0));
    ((ELO_K * (1.0 - expected)).round() as i32).max(1)
}

fn task_title(text: &str) -> String {
    text.lines()
        .map(|l| {
            l.trim()
                .trim_start_matches('#')
                .trim()
                .trim_matches('*')
                .trim()
        })
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(60).collect())
        .unwrap_or_else(|| "Задача".to_string())
}

fn truncate(s: &str, max: usize) -> String {
    let t: String = s.trim().chars().take(max).collect();
    if s.trim().chars().count() > max {
        format!("{t}…")
    } else {
        t
    }
}

struct Verdict {
    kind: &'static str,
    passed: usize,
    total: usize,
    message: String,
}

impl Verdict {
    fn new(kind: &'static str, passed: usize, total: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            passed,
            total,
            message: message.into(),
        }
    }
}

enum Next {
    Nothing,
    Finish,
    NewRound,
}

pub struct BattleService {
    rooms: Arc<BattleManager>,
    problem_service: Arc<ProblemService>,
    piston: Arc<PistonService>,
    redis: Arc<RedisService>,
    repo: Arc<dyn BattleRepository + Send + Sync>,
    user_repo: Arc<dyn UserRepository + Send + Sync>,
}

impl BattleService {
    pub fn new(
        problem_service: Arc<ProblemService>,
        piston: Arc<PistonService>,
        redis: Arc<RedisService>,
        repo: Arc<dyn BattleRepository + Send + Sync>,
        user_repo: Arc<dyn UserRepository + Send + Sync>,
    ) -> Self {
        Self {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            problem_service,
            piston,
            redis,
            repo,
            user_repo,
        }
    }

    async fn get_room(&self, battle_id: Uuid) -> Option<Arc<Mutex<BattleRoom>>> {
        self.rooms.lock().await.get(&battle_id).cloned()
    }

    pub async fn create_room(
        self: &Arc<Self>,
        a: &PlayerInfo,
        b: &PlayerInfo,
    ) -> Result<Uuid, AppError> {
        let battle_id = Uuid::new_v4();
        let level = level_for_elo((a.elo + b.elo) / 2);

        self.repo.create(battle_id, a.id, b.id, level).await?;

        let room = BattleRoom::new(battle_id, level, a, b);
        self.rooms
            .lock()
            .await
            .insert(battle_id, Arc::new(Mutex::new(room)));

        for member in [a, b] {
            tokio::spawn(Arc::clone(self).forfeit_after(battle_id, member.id, JOIN_TIMEOUT));
        }

        Ok(battle_id)
    }

    pub async fn join(
        self: &Arc<Self>,
        battle_id: Uuid,
        user_id: Uuid,
        conn_id: Uuid,
        sender: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<(), AppError> {
        let room = self.get_room(battle_id).await.ok_or(AppError::NotFound)?;

        let mut guard = room.lock().await;

        let me = guard
            .members
            .iter()
            .find(|m| m.id == user_id)
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("Вы не участник этого боя".to_string()))?;
        let opponent = guard
            .members
            .iter()
            .find(|m| m.id != user_id)
            .cloned()
            .ok_or(AppError::NotFound)?;

        guard.players.insert(
            user_id,
            Player {
                conn_id,
                sender: sender.clone(),
            },
        );

        let my_stats = guard.stats_of(user_id);
        let opp_stats = guard.stats_of(opponent.id);
        let _ = sender.send(ServerMsg::BattleInit {
            battle_id,
            level: guard.level,
            me,
            opponent,
            max_hp: START_HP,
            my_hp: my_stats.hp,
            opponent_hp: opp_stats.hp,
            my_score: my_stats.score,
            opponent_score: opp_stats.score,
        });

        if let Some(end) = guard.match_end_for(user_id) {
            let _ = sender.send(end);
            return Ok(());
        }

        let state = guard.task_state;
        match state {
            TaskState::Idle if guard.players.len() < 2 => {
                let _ = sender.send(ServerMsg::WaitingOpponent);
            }
            TaskState::Idle => {
                guard.task_state = TaskState::Generating;
                guard.task_text.clear();
                guard.broadcast(ServerMsg::TaskGenerating);
                tokio::spawn(Arc::clone(self).run_generation(room.clone()));
            }
            TaskState::Generating => {
                let _ = sender.send(ServerMsg::TaskGenerating);
                if !guard.task_text.is_empty() {
                    let _ = sender.send(ServerMsg::TaskChunk {
                        text: guard.task_text.clone(),
                    });
                }
            }
            TaskState::Ready => {
                let _ = sender.send(ServerMsg::TaskReady {
                    text: guard.task_text.clone(),
                });
            }
        }

        Ok(())
    }

    pub async fn leave(self: &Arc<Self>, battle_id: Uuid, user_id: Uuid, conn_id: Uuid) {
        let Some(room) = self.get_room(battle_id).await else {
            return;
        };

        let should_watch = {
            let mut guard = room.lock().await;
            let is_current = guard
                .players
                .get(&user_id)
                .is_some_and(|p| p.conn_id == conn_id);
            if is_current {
                guard.players.remove(&user_id);
            }
            is_current && guard.finish.is_none()
        };

        if should_watch {
            tokio::spawn(Arc::clone(self).forfeit_after(battle_id, user_id, DISCONNECT_GRACE));
        }
    }

    async fn forfeit_after(self: Arc<Self>, battle_id: Uuid, user_id: Uuid, delay: Duration) {
        tokio::time::sleep(delay).await;

        let Some(room) = self.get_room(battle_id).await else {
            return;
        };

        let (winner, reason) = {
            let r = room.lock().await;
            if r.finish.is_some() || r.players.contains_key(&user_id) {
                return;
            }
            match r.opponent_of(user_id) {
                Some(opp) if r.players.contains_key(&opp) => (Some(opp), "disconnect"),
                _ => (None, "abandoned"),
            }
        };

        self.finish_battle(&room, winner, reason).await;
    }

    pub async fn surrender(self: &Arc<Self>, battle_id: Uuid, user_id: Uuid) {
        let Some(room) = self.get_room(battle_id).await else {
            return;
        };

        let winner = {
            let r = room.lock().await;
            if r.finish.is_some() || !r.players.contains_key(&user_id) {
                return;
            }
            r.opponent_of(user_id)
        };

        self.finish_battle(&room, winner, "surrender").await;
    }

    async fn run_generation(self: Arc<Self>, room: Arc<Mutex<BattleRoom>>) {
        let (battle_id, level, round, avoid) = {
            let r = room.lock().await;
            (r.battle_id, r.level, r.round, r.task_names.clone())
        };

        for attempt in 1..=MAX_GENERATION_ATTEMPTS {
            let (tx, mut rx) = mpsc::unbounded_channel::<ProblemEvent>();
            let problem_service = self.problem_service.clone();
            let avoid_for_task = avoid.clone();
            let generation =
                tokio::spawn(
                    async move { problem_service.generate(level, &avoid_for_task, tx).await },
                );

            let mut aborted = false;
            while let Some(event) = rx.recv().await {
                let mut r = room.lock().await;
                if r.finish.is_some() || r.players.is_empty() {
                    aborted = true;
                    break;
                }
                match event {
                    ProblemEvent::Chunk(delta) => {
                        r.task_text.push_str(&delta);
                        r.broadcast(ServerMsg::TaskChunk { text: delta });
                    }
                    ProblemEvent::Verifying => {
                        r.broadcast(ServerMsg::TaskStatus {
                            text: "Проверяем тестовые данные…".to_string(),
                        });
                    }
                }
            }
            drop(rx);

            if aborted {
                generation.abort();
                reset_room(&room).await;
                return;
            }

            match generation.await {
                Ok(Ok(problem)) => {
                    if let Err(e) = self
                        .redis
                        .save_tests(battle_id, round, &problem.tests)
                        .await
                    {
                        tracing::error!(
                            "не удалось сохранить тесты боя {battle_id} в Redis: {e:?}"
                        );
                        break;
                    }

                    let all_tasks = {
                        let mut r = room.lock().await;
                        if r.finish.is_some() {
                            return;
                        }
                        r.task_text = problem.text.clone();
                        r.task_names.push(problem.name.clone());
                        if !r.all_tasks.is_empty() {
                            r.all_tasks.push_str("\n\n---\n\n");
                        }
                        r.all_tasks.push_str(&problem.text);
                        r.task_state = TaskState::Ready;
                        r.broadcast(ServerMsg::TaskReady {
                            text: problem.text.clone(),
                        });
                        r.all_tasks.clone()
                    };

                    if let Err(e) = self.repo.set_task(battle_id, &all_tasks).await {
                        tracing::error!("не удалось сохранить задачу боя {battle_id}: {e:?}");
                    }
                    return;
                }
                Ok(Err(e)) => {
                    println!("генерация задачи (попытка {attempt}) не удалась: {e:?}");
                }
                Err(e) => {
                    println!("генерация задачи (попытка {attempt}) упала: {e:?}");
                }
            }

            if attempt < MAX_GENERATION_ATTEMPTS {
                let mut r = room.lock().await;
                if r.finish.is_some() {
                    return;
                }
                r.task_text.clear();
                r.broadcast(ServerMsg::TaskGenerating);
            }
        }

        fail_generation(&room).await;
    }

    pub async fn submit(
        self: &Arc<Self>,
        battle_id: Uuid,
        user_id: Uuid,
        language: String,
        code: String,
    ) {
        let Some(room) = self.get_room(battle_id).await else {
            return;
        };

        let round = {
            let mut r = room.lock().await;
            if !r.players.contains_key(&user_id) {
                return;
            }
            if r.finish.is_some() {
                r.send_to(user_id, submit_error("Бой уже завершён"));
                return;
            }
            if r.task_state != TaskState::Ready {
                r.send_to(user_id, submit_error("Задача ещё не готова"));
                return;
            }
            if !r.judging.insert(user_id) {
                r.send_to(user_id, submit_error("Предыдущее решение ещё проверяется"));
                return;
            }
            r.send_to(user_id, ServerMsg::Judging);
            r.round
        };

        let verdict = self.judge(battle_id, round, &language, &code).await;

        let mut next = Next::Nothing;
        {
            let mut r = room.lock().await;
            r.judging.remove(&user_id);

            if verdict.kind == "accepted" {
                if r.finish.is_some() || r.round != round {
                    r.send_to(
                        user_id,
                        ServerMsg::SubmitResult {
                            verdict: "late".to_string(),
                            passed: verdict.passed,
                            total: verdict.total,
                            message: "Решение верное, но соперник решил задачу раньше.".to_string(),
                        },
                    );
                } else {
                    let opponent = r.opponent_of(user_id);
                    if let Some(s) = r.stats.get_mut(&user_id) {
                        s.score += 1;
                    }
                    let mut opponent_dead = false;
                    if let Some(opp_stats) = opponent.and_then(|opp_id| r.stats.get_mut(&opp_id)) {
                        opp_stats.hp = (opp_stats.hp - DAMAGE_PER_SOLVE).max(0);
                        opponent_dead = opp_stats.hp == 0;
                    }

                    r.send_to(user_id, verdict_message(&verdict));
                    r.broadcast_state(Some(user_id));

                    if opponent_dead {
                        next = Next::Finish;
                    } else {
                        r.round += 1;
                        r.task_state = TaskState::Generating;
                        r.task_text.clear();
                        r.broadcast(ServerMsg::TaskGenerating);
                        next = Next::NewRound;
                    }
                }
            } else {
                r.send_to(user_id, verdict_message(&verdict));
            }
        }

        match next {
            Next::Nothing => {}
            Next::Finish => self.finish_battle(&room, Some(user_id), "hp_zero").await,
            Next::NewRound => {
                if let Err(e) = self.redis.delete_tests(battle_id, round).await {
                    tracing::warn!("не удалось удалить тесты раунда {round}: {e:?}");
                }
                tokio::spawn(Arc::clone(self).run_generation(room.clone()));
            }
        }
    }

    async fn judge(&self, battle_id: Uuid, round: u32, language: &str, code: &str) -> Verdict {
        let Some((piston_lang, file_name)) = piston_language(language) else {
            return Verdict::new("error", 0, 0, "Этот язык не поддерживается");
        };
        if code.trim().is_empty() {
            return Verdict::new("error", 0, 0, "Пустое решение");
        }
        if code.len() > MAX_CODE_BYTES {
            return Verdict::new("error", 0, 0, "Слишком большой файл с решением");
        }

        let tests = match self.redis.load_tests(battle_id, round).await {
            Ok(Some(tests)) if !tests.is_empty() => tests,
            Ok(_) => return Verdict::new("error", 0, 0, "Тесты для этой задачи не найдены"),
            Err(e) => {
                tracing::error!("не удалось прочитать тесты из Redis: {e:?}");
                return Verdict::new(
                    "error",
                    0,
                    0,
                    "Не удалось получить тесты, попробуйте ещё раз",
                );
            }
        };
        let total = tests.len();

        let runs = join_all(tests.iter().map(|test| async move {
            let stdin = format!("{}\n", test.input);
            self.piston
                .execute(piston_lang, file_name, code, &stdin)
                .await
        }))
        .await;

        let mut passed = 0;
        for (i, run) in runs.into_iter().enumerate() {
            match run {
                Ok(ExecOutcome::CompileError(output)) => {
                    return Verdict::new(
                        "compile_error",
                        0,
                        total,
                        truncate(&output, MAX_MESSAGE_CHARS),
                    );
                }
                Ok(ExecOutcome::Timeout) => {
                    return Verdict::new(
                        "time_limit",
                        passed,
                        total,
                        format!("Тест {}: превышен лимит времени или памяти", i + 1),
                    );
                }
                Ok(ExecOutcome::Finished {
                    stdout,
                    stderr,
                    exit_code,
                }) => {
                    if exit_code != 0 {
                        return Verdict::new(
                            "runtime_error",
                            passed,
                            total,
                            format!(
                                "Тест {}: программа завершилась с кодом {}\n{}",
                                i + 1,
                                exit_code,
                                truncate(&stderr, MAX_MESSAGE_CHARS)
                            ),
                        );
                    }
                    if normalize_output(&stdout) != tests[i].output {
                        return Verdict::new(
                            "wrong_answer",
                            passed,
                            total,
                            format!("Тест {}: неверный ответ", i + 1),
                        );
                    }
                    passed += 1;
                }
                Err(e) => {
                    tracing::error!("Piston недоступен при проверке решения: {e:?}");
                    return Verdict::new(
                        "error",
                        passed,
                        total,
                        "Сервис проверки недоступен, попробуйте ещё раз",
                    );
                }
            }
        }

        Verdict::new("accepted", passed, total, "Все тесты пройдены")
    }

    async fn finish_battle(
        self: &Arc<Self>,
        room: &Arc<Mutex<BattleRoom>>,
        winner: Option<Uuid>,
        reason: &'static str,
    ) {
        let (battle_id, members, scores, rounds) = {
            let mut r = room.lock().await;
            if r.finish.is_some() {
                return;
            }
            r.finish = Some(Finish {
                winner,
                reason,
                elo_delta: HashMap::new(),
            });
            let scores: Vec<i32> = r.members.iter().map(|m| r.stats_of(m.id).score).collect();
            (r.battle_id, r.members.clone(), scores, r.round)
        };

        let mut deltas: HashMap<Uuid, i32> = HashMap::new();
        if let (Some(w), Some(l)) = (
            winner,
            members
                .iter()
                .map(|m| m.id)
                .find(|id| *id != winner.unwrap_or_default()),
        ) {
            let w_elo = self.current_elo(w, &members).await;
            let l_elo = self.current_elo(l, &members).await;

            let gain = elo_change(w_elo, l_elo);
            let loss = gain.min(l_elo);

            deltas.insert(w, gain);
            deltas.insert(l, -loss);

            if let Err(e) = self.user_repo.update_elo(&w, w_elo + gain).await {
                tracing::error!("не удалось обновить ELO победителя {w}: {e:?}");
            }
            if let Err(e) = self.user_repo.update_elo(&l, l_elo - loss).await {
                tracing::error!("не удалось обновить ELO проигравшего {l}: {e:?}");
            }
        }

        let delta_of = |idx: usize| deltas.get(&members[idx].id).copied().unwrap_or(0);
        if let Err(e) = self
            .repo
            .finish(
                battle_id,
                winner,
                scores[0],
                scores[1],
                delta_of(0),
                delta_of(1),
            )
            .await
        {
            tracing::error!("не удалось сохранить итог боя {battle_id}: {e:?}");
        }

        {
            let mut r = room.lock().await;
            if let Some(f) = r.finish.as_mut() {
                f.elo_delta = deltas.clone();
            }
            r.broadcast_match_end();
        }

        for round in 1..=rounds {
            if let Err(e) = self.redis.delete_tests(battle_id, round).await {
                tracing::warn!("не удалось удалить тесты боя {battle_id} (раунд {round}): {e:?}");
            }
        }

        let rooms = self.rooms.clone();
        tokio::spawn(async move {
            tokio::time::sleep(FINISHED_ROOM_KEEP).await;
            rooms.lock().await.remove(&battle_id);
        });
    }

    async fn current_elo(&self, user_id: Uuid, members: &[PlayerInfo]) -> i32 {
        match self.user_repo.find_by_id(&user_id).await {
            Ok(Some(u)) => u.elo,
            _ => members
                .iter()
                .find(|m| m.id == user_id)
                .map(|m| m.elo)
                .unwrap_or(0),
        }
    }
    pub async fn history(&self, user_id: Uuid) -> Result<Vec<BattleHistoryItem>, AppError> {
        let battles = self.repo.find_finished_by_user(user_id).await?;
        let mut items = Vec::with_capacity(battles.len());

        for b in battles {
            let (elo_delta, opponent_id) = if b.user1_id == user_id {
                (b.elo_delta_user1, b.user2_id)
            } else {
                (b.elo_delta_user2, b.user1_id)
            };

            let opponent = self
                .user_repo
                .find_by_id(&opponent_id)
                .await?
                .map(|u| u.username)
                .unwrap_or_else(|| "Удалённый игрок".to_string());

            let result = match b.winner_id {
                Some(w) if w == user_id => "win",
                Some(_) => "loss",
                None => "draw",
            };

            items.push(BattleHistoryItem {
                opponent,
                task: task_title(b.task_text.as_deref().unwrap_or("")),
                result: result.to_string(),
                elo_delta,
                date: b.end_at.unwrap_or(b.start_at),
            });
        }

        Ok(items)
    }
}

fn submit_error(message: &str) -> ServerMsg {
    ServerMsg::SubmitResult {
        verdict: "error".to_string(),
        passed: 0,
        total: 0,
        message: message.to_string(),
    }
}

fn verdict_message(v: &Verdict) -> ServerMsg {
    ServerMsg::SubmitResult {
        verdict: v.kind.to_string(),
        passed: v.passed,
        total: v.total,
        message: v.message.clone(),
    }
}

async fn reset_room(room: &Arc<Mutex<BattleRoom>>) {
    let mut r = room.lock().await;
    if r.finish.is_none() {
        r.task_state = TaskState::Idle;
    }
    r.task_text.clear();
}

async fn fail_generation(room: &Arc<Mutex<BattleRoom>>) {
    let mut r = room.lock().await;
    r.task_state = TaskState::Idle;
    r.task_text.clear();
    r.broadcast(ServerMsg::Error {
        message: "Не удалось сгенерировать задачу. Перезайдите в бой.".to_string(),
    });
}
