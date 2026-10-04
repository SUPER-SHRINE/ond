# Source organization

- When a source file exceeds roughly 1,000 lines, proactively review its responsibilities and split it into cohesive functional modules.
- Prefer ordinary Rust modules with explicit dependencies. Keep public APIs stable during organization-only changes and verify behavior with regression tests.
- Keep tests and implementation separate when doing so makes either easier to navigate; avoid splitting solely to satisfy a line count.
