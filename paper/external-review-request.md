# Request for independent technical review — Caleta preprint

## For Joan-Marc Fisa / BiVelio Inc

Send this request to a researcher with demonstrated experience in at least one relevant area: machine-learning systems, inference systems, software engineering, reproducible research, privacy engineering, or systems security. The reviewer must not be presented as independent if they participated in designing or implementing Caleta, own part of the work, or have another conflict that should be disclosed.

Do not submit the manuscript to arXiv as “externally reviewed” until a real reviewer has provided a substantive response. A review is not peer review by a journal and must not be described that way. Ask permission before publishing the reviewer's name, affiliation, or comments.

## Suggested message

Subject: Independent technical review request — Caleta local-inference preprint

Hello [Name],

I am Joan-Marc Fisa, author of the Caleta preprint at BiVelio Inc. Given your work in [specific relevant research], I would appreciate an independent technical review before we decide whether to submit this manuscript to arXiv.

Caleta is an evidence layer around llama.cpp for artifact verification, controlled local execution, restricted API exposure, and reproducible measurement. The current version includes deterministic tests on three operating systems and a pinned real-GGUF experiment on one Windows CPU host. We are not claiming a new inference engine, universal performance improvement, absolute privacy, or journal peer review.

Materials:

- Manuscript source and PDF: https://github.com/BiVelio/caleta/tree/main/paper
- Real-model methodology and evidence: https://github.com/BiVelio/caleta/tree/main/experiments/real-gguf-qwen2.5-0.5b
- Implementation: https://github.com/BiVelio/caleta
- Earlier Zenodo version: https://doi.org/10.5281/zenodo.23761755

Please focus on errors or unsupported claims rather than stylistic approval. In particular:

1. Is the contribution clearly distinguished from llama.cpp and existing inference runtimes?
2. Do the abstract, research questions, results, and conclusion overstate what the evidence supports?
3. Are the mock and real-model experiments properly separated?
4. Are the single-host measurements, caching caveat, aggregation, and limitations technically sound?
5. Are integrity, declared provenance, authenticated provenance, privacy, and attestation used accurately?
6. Are any references inaccurate, missing essential context, or misrepresented?
7. Can the public artifacts and commands support an independent repetition?
8. What specific changes would you require before public submission?

A useful response would identify major and minor issues and end with one recommendation: submit, submit after specified changes, or do not submit in the current form. Please include any conflicts of interest and the basis for your technical assessment. You are welcome to report that you have not checked a particular claim or have not run the code.

If you agree, also tell us whether we may publish your review, name, and affiliation alongside the manuscript. We will not publish them without explicit permission, and we will preserve the substance of any criticism.

Thank you,
Joan-Marc Fisa
BiVelio Inc
jmfisa@bivelio.com

## Review record — complete only after a real response

- Reviewer name:
- Qualifications and relevant research:
- Affiliation:
- Relationship or conflicts of interest:
- Materials and exact revisions reviewed:
- Did the reviewer run the code or only inspect it?
- Major issues identified:
- Minor issues identified:
- Actions taken by the authors:
- Remaining disagreements:
- Recommendation:
- Permission to publish review/name/affiliation:
- Review date and permanent link, if published:

Until this record is completed with verifiable information, the manuscript has **not received independent external review**.
