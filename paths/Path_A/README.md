## Path A (clinical KPs)

### Rare Pulmonary Disease Use Case

Path A was developed by the TCDC in support of MVP1 (_what drugs may treat disease X_) and leveraging clinical KP "knowledge". Development and testing focused on a select set of CURIEs for rare pulmonary diseases plus a common pulmonary disease (asthma) and a non-pulmonary disease (EDS) for comparison. 

**Input Disease CURIES for development and testing:**

- primary ciliary dyskinesia (MONDO:0016575)
- cystic fibrosis (MONDO:0009061)
- idiopathic bronchiectasis (MONDO:0018956)
- lymphangioleiomyomatosis (MONDO:0011705)
- idiopathic pulmonary fibrosis (MONDO:0008345)
- asthma (MONDO:0004979)
- EDS (MONDO:0020066)

**SMEs**

- Dr. Michael Knowles, UNC Chapel Hill
- Dr. Margaret Leigh, UNC Chapel Hill

### Overall Structure

Path A serves as a CQS template and is structured as a valid TRAPI query, with workflow parameters and an _allowlist_ parameter. 

Note that Path A targets the clinical KPs, using the following predicates plus an _allowlist_ parameter:

    associated_with
    
      associated_with_increased(decreased)_likelihood_of

      correlated_with

        positively_correlated_with

        negatively_correlated_with

Path A of the CQS MVP1 workflow is intended to first identify biolink:ChemicalEntity associated with a biolink:Disease input CURIE for a rare pulmonary disease and targeting the clinical KPs (COHD, icees-kg, Multiomics EHR Risk Provider), then identify biolink:Gene affected by those chemical entities, and finally identify biolink:Drug that affects the same gene set and is also related to the input disease CURIE. Note that the original intent was to only include drugs that are not in the first set of chemical entities, but that plan changed.
