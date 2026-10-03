#!/bin/bash

for p in python gcc java rust node typescript go kotlin swift ruby php dotnet; do
  echo "== $p"
  curl -s -X POST http://localhost:2000/api/v2/packages \
    -H "Content-Type: application/json" \
    -d "{\"language\":\"$p\",\"version\":\"*\"}"
  echo
done
