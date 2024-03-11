# README

## Translator Curated Query Service (CQS) - CQS MVP Templates

This folder is intended to suppport manually-defined, SMuRF- and SME-evaluated inferred CQS workflows, with each workflow structured as a valid TRAPI query and serving as a CQS template.

The CQS was conceptualized by the Translator Clinical Data Committee, but initial development and implementation were conducted under the Standards and Reference Implementation (SRI) component of Translator. The CQS provides a simple mechanism through which KP teams or any committee, working group, or external team can apply their expertise /resources to specify how their data are to be used for inference. Thus, the CQS enables a ”conservative ingest” paradigm, where KP teams directly ingest knowledge sources and perhaps process them, but rely on the CQS services to generate desired inferences based on this more foundational knowledge.

The process to contribute a new QQS template is as follows:

1. Develop a set of rules for how a particular KP can contribute to an inferred MVP query.
2. Specify a valid TRAPI query that can serve as a CQS template.
3. Test the CQS template by direct query of the KP.
4. Working within a branch, create a new template folder within CQS/templates; within that folder, add a thoroughly descriptive README with a POC and deposit the new CQS template or valid TRAPI; create a PR.
5. The new CQS template will then be deployed to DEV, thus entering the Translator pipeline.

The nomenclature for CQS templates is as follows:

Human-readable format: MVP# Template # (infores)
Example: MVP1 Template 3 (openpredict)

GitHub format: mvp#-template#-infores

Example JSON query



