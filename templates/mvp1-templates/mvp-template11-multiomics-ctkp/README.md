## Description

This CQS template generates `biolink:treats` predictions based on `biolink:in_clinical_trials_for` assertions in the Multiomics Clinical Trials KP, with an `elevate_to_prediction` tag. The KP adds this tag for assertions with `max_research_phase` of at least phase 1, and less than phase 4.

## Testing

The following CURIEs should produce results:
```
MONDO:0008170
MONDO:0008223
MONDO:0000477
MONDO:0003233
```
