# Preprint submission metadata

Use the same scholarly version and metadata on arXiv, Zenodo, and SSRN. Do not create three different titles or claim three independent studies. Fields not yet known, including ORCID and external DOIs, must be supplied only after verification.

## Common metadata

- **Title:** Caleta: An Evidence-Oriented Trust Layer for Reproducible Local Large Language Model Inference
- **Author:** Joan-Marc Fisa
- **Affiliation:** BiVelio Inc
- **Corresponding author:** Joan-Marc Fisa
- **Email:** jmfisa@bivelio.com
- **Copyright holder:** BiVelio Inc
- **Manuscript license:** Creative Commons Attribution 4.0 International (CC BY 4.0)
- **Software license:** Apache License 2.0
- **Publication type:** Preprint / technical manuscript; not peer reviewed
- **Language:** English
- **Abstract:** use `abstract.txt`; it is ASCII-only and shorter than arXiv's 1,920-character limit
- **Keywords:** local inference; large language models; reproducibility; software supply chain; privacy engineering; benchmarking; provenance; llama.cpp
- **Code and artifacts:** https://github.com/BiVelio/caleta
- **Validation evidence:** https://github.com/BiVelio/caleta/actions/runs/40758287817
- **Zenodo v1.0:** published 29 October 2026 at https://zenodo.org/records/23761755 — DOI 10.5281/zenodo.23761755.
- **Zenodo v1.1:** draft 23763881 with reserved DOI 10.5281/zenodo.23763881; not registered until publication.
- **Zenodo all-versions DOI:** 10.5281/zenodo.23761754

## arXiv

- **Submission account:** Joan-Marc Fisa's own account. arXiv expects authors to self-submit; third-party submission requires prior authorization.
- **Primary category:** cs.SE (Software Engineering).
- **Cross-list:** cs.CL (Computation and Language), if the submission form permits and the author agrees with the topical fit.
- **Authors field:** Joan-Marc Fisa (BiVelio Inc)
- **Comments:** 13 pages, 1 figure, 3 tables. Non-peer-reviewed technical preprint with real-GGUF validation. Reference implementation and reproducibility materials: https://github.com/BiVelio/caleta and https://github.com/BiVelio/caleta/tree/main/experiments/real-gguf-qwen2.5-0.5b
- **Files:** upload the LaTeX source package, not only the locally generated PDF. arXiv prefers TeX/LaTeX and does not accept PDF generated from TeX as a substitute for the source.
- **License:** CC BY 4.0. The selected license is irrevocable for that arXiv version.
- **Journal reference and external DOI:** leave empty unless a genuine corresponding publication identifier exists. Do not put a future arXiv identifier in these fields.
- **Endorsement:** may be requested for a new author or category. Use the institutional email and, if necessary, a qualified endorser who knows the author or work.

## Zenodo

- **Resource type:** Publication -> Preprint.
- **Creators:** Fisa, Joan-Marc; affiliation BiVelio Inc.
- **Files:** final PDF, LaTeX source, plain-text abstract, submission metadata, and CC BY 4.0 license text.
- **License:** CC-BY-4.0.
- **Related identifier:** https://github.com/BiVelio/caleta with relation `isSupplementedBy` and resource type `software`.
- **DOI:** reserve it in the draft if the DOI should appear inside the PDF. Regenerate the PDF with the reserved value before publishing. Never invent or guess a DOI.
- **Version:** v1.1-preprint (adds real-GGUF validation and academic related work).
- **Publication date:** use the actual public deposit date.
- **Visibility:** public. Preview the record before publishing.

The accompanying `zenodo-metadata.json` is an API-oriented metadata draft. It intentionally contains no access token, ORCID, or unverified DOI.

## SSRN

- **Submitter:** Joan-Marc Fisa or an explicitly authorized BiVelio account with the right to grant distribution permission.
- **File:** final PDF with searchable text.
- **Title, abstract, author, affiliation, and keywords:** use the common metadata above.
- **Topic selection:** choose the closest available categories for software engineering, artificial intelligence, and information systems after inspecting the current form; do not invent fixed category identifiers.
- **Distribution:** SSRN's submission instructions state that authors retain copyright and grant SSRN a non-exclusive right to post and distribute the paper. Read the current submission agreement before accepting it.
- **Other versions:** add the Zenodo DOI and arXiv identifier only after they really exist.

## Before each final submission

1. Confirm Joan-Marc Fisa consents to the submission and that BiVelio Inc authorizes CC BY 4.0 for the manuscript.
2. Confirm that the title, abstract, author, affiliation, license, and embedded identifiers match exactly across platforms.
3. Inspect the final PDF, not only the LaTeX source.
4. Keep the statements about mock-based validation and absent real-GGUF experiments unchanged.
5. Save confirmation screens, identifiers, and submission dates.
6. After publication, add only the real identifiers to this file and the repository; do not replace a public arXiv submission with a duplicate when an update is required.
