# README

## Translator Curated Query Service

This folder is intended to suppport development of a Translator Clinical Data Committee (TCDC) drug discovery/repurposing get_creative() workflow, or a minimum set of SME-informed, curated, TRAPI queries.

TCDC partnered originally with Team ARAX to execute this workflow as a TCDC get_creative() 'ARA'. Specifically, the initial plan was for Team ARAX to standup an 'ARA' endpoint to support the workflow. This would have allowe the TCDC 'ARA' endpont to respond to ARS get_creative() MVP1 TRAPI queries during the SME - UI relay sessions at the SEP2022 relay meeting. The intent was to structure the queries as a high-level ‘templated’ question: 'using whatever creative means necessary, find me drugs that may treat disease X' or 'what drugs may treat disease X?’.

However, after the SEP2022 relay meeting, the TCDC and the SRI team discussed whether it makes sense for the TCDC to stand up their own skeletal ARA, the Translator Curated Query Serivce (CQS), and for this effort to move under the umbrella of the SRI for development and long-term maintenance, potentially serving as an exemplar for other committees/WGs/teams to stand up their own specialized 'ARA' to execute curated, SME-informed workflows in response to inferred MVP queries. The decision was made to move forward with the Translator CQS. See [issue #10](https://github.com/NCATSTranslator/Clinical-Data-Committee-Tracking-Voting/issues/17) for details. Jason R. is leading the development of the Translator CQS.

In JAN2024, after extensive discussions with the Biolink/EPC team, the purpose of the CQS was extended to support one-hop KP-derived predictions. The overall idea is that the CQS will:

1. Support manually-defined, SMuRF- and SME-evaluated inference workflows to be contributed by any team or working group, or even external groups.

2. Provide simple mechanism through which KPs can apply their expertise /resources to specify how their data are to be used for inference. This can enable a ”conservative ingest” paradigm - where KPs ingest what sources directly assert and rely on CQS services to generate desired inferences based on this more foundational knowledge.

3. Allow KP teams such as OpenPredict or Multiomics to avoid dealing with ARA functions such as aux graphs, ARS registration, merging, scoring, normalizing, adding literature co-occurrence.
   
4. Facilitate consistent specification and implementation of inference rules, by providing a centralized and transparent place to define, align, and collaborate on inference rules.

Numerous use cases have been put forward, e.g., see [slides](https://docs.google.com/presentation/d/1mwoPT0IZcY5-TUlflPMmLbrkRT5DpSLRPfuYYhl51Lk/edit?usp=sharing). While there remain concerns regarding scalability and sustainability of the proposed CQS solution, the plan is to move forward with development.

For the initial CDC get_creative() workflow, disease X = rare pulmonary disease.

### Rare Pulmonary Diseases

Three initial CQS SMuRF/SME-developed workflows (Paths A, B, E) were developed by the TCDC in support of MVP1 (_what drugs may treat disease X_). Development and testing focused on a select set of CURIEs for rare pulmonary diseases plus a common pulmonary disease (asthma) and a non-pulmonary disease (EDS) for comparison. The relevant CURIES for which the workflows should be able to respond to include the following:

- primary ciliary dyskinesia (MONDO:0016575)
- cystic fibrosis (MONDO:0009061)
- idiopathic bronchiectasis (MONDO:0018956)
- lymphangioleiomyomatosis (MONDO:0011705)
- idiopathic pulmonary fibrosis (MONDO:0008345)
- asthma (MONDO:0004979)
- EDS (MONDO:0020066)

### SMEs

- Dr. Michael Knowles, UNC Chapel Hill
- Dr. Margaret Leigh, UNC Chapel Hill

### Overall Structure

The current CQS workflow is comprised of three main paths, as described in greater detail under directories Path_A, Path_B, and Path_E, as well as in this [slide deck](https://docs.google.com/presentation/d/1pQp4SC9xxHojFdm1H4z_mdHSi6wpv7pq/edit?usp=sharing&ouid=112054006232285231595&rtpof=true&sd=true). Each path is a TRAPI query, which serves as the configuration file for the CQS.

Note that Path A targets the clinical KPs, using the following predicates plus an _allowlist_ parameter:

    associated_with
    
      associated_with_increased(decreased)_likelihood_of

      correlated_with

        positively_correlated_with

        negatively_correlated_with


![image](https://github.com/TranslatorSRI/CQS/assets/26254388/f06ec0ef-d2bc-45ec-bdc8-d0feae58933c)



