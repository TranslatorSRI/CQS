# Curated Query Service (CQS)

## CQS Overview

The CQS was conceptualized by the Translator Clinical Data Committee (TCDC) in Fall 2022. The goal is to create a skeletal ARA that initially will support the [TCDC's MVP1 workflow on rare pulmonary disease](https://github.com/TranslatorSRI/CQS/tree/main/paths), e.g., MVP1 Template 1 (clinical-kps), but the goal is for the CQS to provide a general model and approach for other teams, committees, working groups, and external users who wish to contribute to the Translator ecosystem. The development and implementation work is being supported by the Translator Standards and Reference Implementation (SRI) core, with Jason Reilly serving as lead developer. Plans for long-term maintenance are TBD.

**What It Does**

1. An SRI Service that provides ARA-like capabilities:
   
- generation of ‘predicted’ edges in response to creative queries - based on customizable inference rules

- linking predictions to their supporting aux graphs

- attachment of provenance metadata and scores to results

2. Inference specifications are defined as TRAPI templates, which serve as config files for a custom reasoning service / workflow 

3. Scoring of individual workflow templates can be customized

- e.g., ARAGORN’s scoring/ranking algorithm, OpenPredict’s prediction score 

- Scoring within a result is in descending order, based on the analysis score. Scoring across results is currently based on the max analysis score, in descending order

**What it Enables**

1. Supports manually-defined, SMuRF- and SME-evaluated inferred workflows to be contributed by any team or working group, or even external groups; each workflow is structured as a valid TRAPI query and serves as a CQS template

2. Provides simple mechanism through which KPs can apply their expertise /resources to specify how their data are to be used for inference
- This can enable a ”conservative ingest” paradigm - where KPs ingest what sources directly assert and rely on CQS services to generate desired inferences based on this more foundational knowledge

2. Allows KP teams such as OpenPredict or Multiomics to avoid dealing with ARA functions such as aux graphs, ARS registration, merging, scoring, normalizing, adding literature co-occurrence
   
4. Facilitates consistent specification and implementation of inference rules, by providing a centralized and transparent place to define, align, and collaborate on inference rules

## Architectural Overview

![image](https://github.com/TranslatorSRI/CQS/assets/26254388/c8989e81-a3b3-48e6-b2a0-f43e0352412e)

## CQS Implementation Plan

A detailed implementation plan was developed by Jason F., Arbrar M., Chris B., Casey T., and Kara F. on 11/15/2022 and finalized by those same persons on 11/17/2022. That plan is described below.

- Jason will register within CQS mappings between an MVP template query-graph and one or more TRAPI queries with workflows but without score operations (i.e., a valid TRAPI message with a query_graph and a workflow element)  
  - For the ‘treats’ MVP1 question, there will be [three templates, MVP1 Template 1 (clinical-kps), MVP1 Template 2 (connections-hypothesis), and MVP1 Template 3 (openpredict)](https://github.com/TranslatorSRI/CQS/tree/main/paths), for initial deployment and testing, with additional workflow templates implemented after validation of the service
  - The MVP1 Templates 1, 2, and 3 will be configured as valid TRAPI queries, with allowlist parameters to target Aragorn and select KPs, depending on the template.
- At runtime, when the registered template query-graph (without a workflow but with a URL for return response) comes in from the ARS, the CQS will submit the associated TRAPI queries with workflows but without score operations to the Workflow Runner (WFR) and get back the results
- After all results are returned, the CQS will use FastAPI Reasoner Pydantic to merge the N sets of results by the result node
- The CQS will then score results using [Aragorn's scoring/ranking operation](https://github.com/ranking-agent/aragorn-ranker) (Templates 1 and 2) or OpenPredict's scoring metric (Template 3)
- If a result is supported by more than one templateh, then the CQS will include the max analysis score in the full result; other options for future consideration include a self-weighted mean: (score = sum(score_i^2) / sum(score_i) [heavier weight on higher scores]
- After scoring results, the CQS will return the results to the ARS in the form of inferred treats edges with supporting aux graphs, i.e, Templates 1, 2, 3 

Please refer to the [wiki](https://github.com/TranslatorSRI/CQS/wiki) for more detailed technical documentation (to be added).

