# Experiment records

The numbered files in this directory are immutable experimental design
records. They preserve hypotheses, modifications, tests, measurements,
decisions, artifact paths and hashes, and links to the research that motivated
the work.

The active source tree is not an archive of every rejected implementation.
After an experiment is complete, one-off binaries and benchmark harnesses may
be removed when all of the following are true:

- the experiment is rejected or superseded;
- the retained record is sufficient to identify the tested implementation,
  result artifact, and source history;
- no current evaluation, frontier, profiling, or compatibility path depends
  on the tooling; and
- removing it does not remove a decoder for an emitted stream version.

Use the experiment's source commit or Git history to reproduce retired
tooling. This keeps the current CPU and future GPU implementation surface
focused without rewriting historical evidence.
