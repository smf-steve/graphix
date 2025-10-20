# Modules

Graphix has a simple module system, with the minimal required functionality
necessary to organize code into a project.

The module system should be considered a work in progress, more features may be
added in the future. In particular there are a few unfinished parts,

- no module renaming on use
- no access control, everything in a module is currently public

These shortcomings may be fixed in a future release. Current features include,

- module hierarchies
- inline modules (defined in the same file)
- modules defined in external files
- modules defined in netidx
- modules dynamically loadable at runtime
