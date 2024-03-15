# README

## Translator Curated Query Service (CQS) - CQS MVP Templates

This folder is intended to suppport manually-defined, SMuRF- and SME-evaluated inferred CQS workflows, with each workflow structured as a valid TRAPI query and serving as a CQS template.

The CQS was conceptualized by the Translator Clinical Data Committee, but initial development and implementation were conducted under the Standards and Reference Implementation (SRI) component of Translator. The CQS provides a simple mechanism through which KP teams or any committee, working group, or external team can apply their expertise /resources to specify how their data are to be used for inference. Thus, the CQS enables a ”conservative ingest” paradigm, where KP teams directly ingest knowledge sources and perhaps compute on them, but rely on the CQS service to generate desired inferences based on this more foundational knowledge.

**The process to contribute a new QQS template is as follows:**

1. Develop a set of "rules" specifying when a particular KP can contribute to an inferred MVP query.
2. Apply the rules in (1) via a valid TRAPI query that can serve as a CQS template.
3. Test the CQS template by direct query of the KP.
4. Create a branch in the CQS repo.
- Add a new template folder within CQS/templates.
- Within that folder, add a thoroughly descriptive README with a POC and select CURIES to be used for development and testing. Those CURIES should be associated with test assets that the KP developers contributed to the test assets repo, using [this G-sheet](https://docs.google.com/spreadsheets/d/1wAQaFEtFqAvp2fbTZIe-2ObF9zUU_cmXILfU8SzUWe0/edit?usp=drive_link).
- Also add a new CQS template structured as a valid TRAPI.
- Create a PR.
6. The new CQS template will then be deployed to DEV, thus entering the Translator pipeline.
7. After the CQS is deployed to CI, it will be picked up by the Information Radiator for automated testing. The POC for a given CQS template is responsible for monitoring the testing results.

*See https://github.com/NCATSTranslator/OperationsAndWorkflows/tree/main/schema for valid TRAPI operations and workflows.*

**The nomenclature for CQS templates is as follows:**

Human-readable format: MVP# Template # (infores or otherwise short but descriptive name that captures the intent of the template)
Example: MVP1 Template 3 (openpredict)

GitHub format: mvp#-template#-infores or mvp#-template#-descriptive-name

[Example JSON query](https://github.com/TranslatorSRI/CQS/tree/karafecho-patch-2/templates/example-cqs-mvp-template)



