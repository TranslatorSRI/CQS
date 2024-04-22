# Curated Query Service (CQS)

## CQS Overview

The CQS was conceptualized by the Translator Clinical Data Committee (TCDC) in Fall 2022. The goal is to create a skeletal ARA that initially will support the [TCDC's MVP1 workflow on rare pulmonary disease](https://docs.google.com/presentation/d/1pQp4SC9xxHojFdm1H4z_mdHSi6wpv7pq/edit?usp=sharing&ouid=112054006232285231595&rtpof=true&sd=true), e.g., MVP1 Template 1 (clinical-kps), but the intent is for the CQS to provide a general model and approach for other teams, committees, working groups, and external users who wish to contribute to the Translator ecosystem. The development and implementation work is being supported by the Translator Standards and Reference Implementation (SRI) core, with Jason Reilly serving as lead developer. Plans for long-term maintenance are TBD.

### What It Does

1. An SRI Service that provides ARA-like capabilities:
   
- generation of ‘predicted’ edges in response to creative queries - based on customizable inference rules

- linking predictions to their supporting aux graphs

- attachment of provenance metadata and scores to results

2. Inference specifications are defined as TRAPI templates, which serve as config files for a custom reasoning service / workflow

- The specifications include a required field to specify knowledge level / agent type (e.g., "resource_id": "infores:biothings-explorer", "resource_role": "primary_knowledge_source") and optional fields to specify, for example, workflow parameters such as an "allow list"

4. Scoring of individual workflow templates can be customized

- e.g., ARAGORN’s scoring/ranking algorithm, OpenPredict’s prediction score 

- Scoring within a result is in descending order, based on the analysis score. Scoring across results is currently based on the max analysis score, in descending order

### What it Enables

1. Supports manually-defined, SMuRF- and SME-evaluated inferred workflows to be contributed by any team or working group, or even external groups; each workflow is structured as a valid TRAPI query and serves as a CQS template

2. Provides simple mechanism through which KPs can apply their expertise /resources to specify how their data are to be used for inference
- This can enable a ”conservative ingest” paradigm - where KPs ingest what sources directly assert and rely on CQS services to generate desired inferences based on this more foundational knowledge

2. Allows KP teams such as OpenPredict or Multiomics to avoid dealing with ARA functions such as aux graphs, ARS registration, merging, scoring, normalizing, adding literature co-occurrence
   
4. Facilitates consistent specification and implementation of inference rules, by providing a centralized and transparent place to define, align, and collaborate on inference rules

## How to Contribute a CQS Template and enter it into the Translator pipeline

See README in "templates directory": https://github.com/TranslatorSRI/CQS/tree/main/templates.

## Architectural Overview

![image](https://github.com/TranslatorSRI/CQS/assets/26254388/c8989e81-a3b3-48e6-b2a0-f43e0352412e)

## CQS Implementation Plan

The initial implememtation plan for the CQS can be found here: https://github.com/NCATSTranslator/Clinical-Data-Committee-Tracking-Voting/issues/17.

Please refer to the [wiki](https://github.com/TranslatorSRI/CQS/wiki) for more detailed technical documentation.

