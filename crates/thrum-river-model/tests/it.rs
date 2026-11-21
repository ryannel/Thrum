//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod support;

mod brink_end;
mod brinks;
mod chart;
mod chop;
mod closed_forms;
mod conformance;
mod energy;
mod fall_gauge;
mod falls;
mod flow_bands;
mod fronts;
mod gauge;
mod plunge;
mod report;
mod samples;
mod sheet;
mod store;
mod valley;
mod wetting;
