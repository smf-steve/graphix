# Modules

Graphix has a simple module system, with the minimal required functionality
necessary to organize code into a project.

The module system should be considered a work in progress, more features may be
added in the future. In particular there are a few unfinished parts of the
module system,

- no forward declarations, modules may only refer to modules already declared in
  their scope
- no access control, everything in a module is currently public
