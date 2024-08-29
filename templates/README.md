# README

## Translator Curated Query Service (CQS) - CQS MVP Templates

This folder is intended to suppport manually-defined, SMuRF- and SME-evaluated inferred CQS workflows, with each workflow structured as a valid TRAPI query and serving as a CQS template.

The CQS was conceptualized by the Translator Clinical Data Committee, but initial development and implementation were conducted under the Standards and Reference Implementation (SRI) component of Translator. The CQS provides a simple mechanism through which KP teams or any committee, working group, or external team can apply their expertise /resources to specify how their data are to be used for inference. Thus, the CQS enables a ”conservative ingest” paradigm, where KP teams directly ingest knowledge sources and perhaps compute on them, but rely on the CQS service to generate desired inferences based on this more foundational knowledge. For instance, the CQS templates are used by the CQS to generate "treats" predictions based on a set of rules developed by the contributing KPs who expose the primary knowledge source (e.g., Clinical Trials provider exposes one-hop in_clinical_trials_for edges and directs the CQS to generate a predicted "treats" edge when a clinical trial meets certain criteria such as in phase 3 or 4).

**The process to contribute a new CQS template is as follows:**

1. Develop a set of "rules" specifying when a particular KP can contribute to an inferred MVP query.
2. Apply the rules in (1) via a valid TRAPI query that can serve as a CQS template.
   - Include required specifications such as a field specifying primary and aggregator knowledge sources (see [example template](https://github.com/TranslatorSRI/CQS/blob/main/templates/example-cqs-mvp-template/example-cqs-mvp-template.json)).
   - Include an "id" field for n0 in the form of an empty array.
   - Include any additional specifications such as attribute constraints and workflow parameters such as an "allowlist".
4. Test the CQS template by direct query of the Workflow Runner.
5. Create a branch in the CQS repo.
   - Create a new template folder within CQS/templates. Following the nomenclature specified below.
   - Within that folder, add a thoroughly descriptive README with a POC and select CURIES to be used for development and testing. The CURIES should be associated with test assets that the POC has contributed to the test assets repo: https://github.com/NCATSTranslator/Tests.
   - Add a new CQS template structured as a valid TRAPI.
   - Create a PR.
5. The new CQS template will then be deployed to DEV, thus entering the Translator pipeline.
6. After the CQS is deployed to CI, it will be picked up by the Information Radiator for automated testing. **The POC for a given CQS template is responsible for monitoring the testing results.**

*See https://github.com/NCATSTranslator/OperationsAndWorkflows/tree/main/schema for valid TRAPI operations and workflows.*

**The nomenclature for CQS templates is as follows:**

Human-readable format: MVP# Template # (infores or otherwise short but descriptive name that captures the intent of the template)
Example: MVP1 Template 3 (openpredict)

GitHub format: mvp#-template#-infores or mvp#-template#-descriptive-name

**Note that MVP2 templates should be named as follows: MVP2-up-gene, MVP2-down-gene, MVP2-up-chemical, MVP2-down-chemical.**

[Example JSON query](https://github.com/TranslatorSRI/CQS/blob/main/templates/example-cqs-mvp-template/example-cqs-mvp-template.json)

**If you need edit access to the CQS repo, please contact Tursynay.**



