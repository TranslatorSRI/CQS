#!/bin/bash

BASE_URL="https://raw.githubusercontent.com/NCATSTranslator/Clinical-Data-Committee-Tracking-Voting/main/GetCreative()_DrugDiscoveryRepurposing_RarePulmonaryDisease"
paths=("$BASE_URL/Path_A/Path_A_e1-e2-allowlist.json,n0,path_a.template.json", "$BASE_URL/Path_B/Path_B_TRAPI.json,n0,path_b.template.json", "$BASE_URL/Path_E/path_e_query.json,n1,path_e.template.json")
for i in ${paths[@]}; do
  IFS="," read url node output <<< "${i}"
  wget -qO- $url | jq '.message.query_graph.nodes."'"$node"'".ids = ["CURIE_TOKEN"]' | sed -e 's;\"CURIE_TOKEN\";CURIE_TOKEN;g' > src/data/$output
done
