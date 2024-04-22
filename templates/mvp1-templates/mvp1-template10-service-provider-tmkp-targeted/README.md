## Description

This CQS template is designed to capture `chemical - biolink:treats_or_applied_or_studied_to_treat - disease_or_phenotypic_feature` assertions that are sourced in the Text Mining Targeted KP.

Caveats:
* The Workflow Runner cannot query the TMKP Targeted KG directly. It queries the general Service Provider end point in order to retrieve TMKP Targeted assertions. The current template is not constrained to TMKP results specifically, so it could possibly return results from other Service Provider services.
* The template requires at least 5 evidence sentences to support an assertion in order for it to be included in the result set.


## Testing

The following assertion exists in the TMKP Targeted Assertion KG:
`DRUGBANK:DB15223 (Flotetuzumab) -- biolink:treats_or_applied_or_studied_to_treat --> MONDO:0018874 (acute myeloid leukemia)`