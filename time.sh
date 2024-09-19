#!/bin/bash

entries=("asthma,MONDO:0004979", "cystic_fibrosis,MONDO:0009061", "mvp1,MONDO:0024529", "malignant_neoplasm,MONDO:0008170")

#HOST="http://localhost:8000"
#HOST="https://cqs-dev.apps.renci.org"
HOST="https://cqs.ci.transltr.io"

for i in ${entries[@]}; do
  IFS="," read disease curie <<< "${i}"

  SECONDS=0

  jq ".message.query_graph.nodes.n1.ids = [\"$curie\"]" sample_input_async.json > /tmp/$disease-input.json

  JOB_ID=`curl -s -X POST "$HOST/asyncquery" -d @/tmp/$disease-input.json -H 'Content-Type: application/json' -H 'Accept: application/json' | jq ".job_id" | sed 's;\";;g'`

  JOB_STATUS=`curl -s -X GET "$HOST/asyncquery_status/$JOB_ID" | jq ".status" | sed 's;\";;g'`
  echo "$JOB_ID is $JOB_STATUS"
  while [ "$JOB_STATUS" != "Completed" ]; do
    sleep 10
    JOB_STATUS=`curl -s -X GET "$HOST/asyncquery_status/$JOB_ID" | jq ".status" | sed 's;\";;g'`
    echo "$JOB_ID is $JOB_STATUS"
  done

  curl -s -X GET "$HOST/download/$JOB_ID" | jq > /tmp/sample_output_$disease.pretty.json
  duration=$SECONDS
  echo "$((duration / 60)) minutes and $((duration % 60)) seconds elapsed."

done
