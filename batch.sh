#!/bin/bash

entries=("asthma,MONDO:0004979", "primary_ciliary_dyskinesia,MONDO:0016575", "cystic_fibrosis,MONDO:0009061", "idiopathic_bronchiectasis,MONDO:0018956", "lymphangioleiomyomatosis,MONDO:0011705", "idiopathic_pulmonary_fibrosis,MONDO:0008345", "EDS,MONDO:0020066")
for i in ${entries[@]}; do
  IFS="," read disease curie <<< "${i}"
  jq ".message.query_graph.nodes.n1.ids = [\"$curie\"]" sample_input.json > /tmp/$disease-input.json
  #curl -X POST https://cqs-dev.apps.renci.org/v0.2/query -d @/tmp/$disease-input.json -H 'Content-Type: application/json' -H 'Accept: application/json' | jq > /tmp/$disease-output.pretty.json
  curl -X POST http://localhost:8000/query -d @/tmp/$disease-input.json -H 'Content-Type: application/json' -H 'Accept: application/json' | jq > /tmp/$disease-output.pretty.json
done




