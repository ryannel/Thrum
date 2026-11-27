use std::fmt::Write as _;
use thrum_render::graph::{CompiledGraph, Lifetime, ResourceId};

const RESOURCE: &str = "ResourceId { index: ";

fn index(resource: &ResourceId) -> usize {
    format!("{resource:?}")
        .strip_prefix(RESOURCE)
        .and_then(|rest| rest.split_once(','))
        .expect("resource index in the historical snapshot format")
        .0
        .parse()
        .expect("numeric graph resource index")
}

/// Project the split atmosphere light buffers back onto the single imported
/// resource used by the historical graph fixtures.
pub struct BeforeLightSplit {
    environment: usize,
}

impl BeforeLightSplit {
    /// Find the buffer written by the diffuse pass. Its whole 160-byte range
    /// distinguishes the environment buffer from that pass's texture hazards.
    /// # Panics
    /// Panics if the fixture lacks a live diffuse pass or its environment buffer.
    pub fn new(graph: &CompiledGraph) -> Self {
        let diffuse = graph
            .pass_names()
            .position(|name| name == "atmosphere diffuse irradiance")
            .expect("atmosphere in the historical fixture");
        let mut outputs = graph
            .hazards
            .iter()
            .filter(|hazard| hazard.before == graph.pass_order[diffuse] && hazard.range == (0..160))
            .map(|hazard| &hazard.resource);
        let environment = outputs.next().expect("the diffuse output is read");
        assert!(outputs.all(|resource| resource == environment));
        Self {
            environment: index(environment),
        }
    }

    /// Replace the two imported light lifetimes with their historical union.
    /// Preserve the relative order and every field of the other lifetimes.
    /// # Panics
    /// Panics if the two expected imported light lifetimes are absent or transient.
    pub fn lifetimes(&self, graph: &CompiledGraph) -> Vec<Lifetime> {
        // The directional record is declared before the view and view-aux
        // uniforms. The split added the environment record after those two.
        let directional = self.environment - 3;
        let find = |id| {
            graph
                .lifetimes
                .iter()
                .find(|life| index(&life.resource) == id)
                .expect("imported light lifetime")
        };
        let mut light = find(directional).clone();
        let environment = find(self.environment);
        assert_eq!(light.transient_slot, None);
        assert_eq!(environment.transient_slot, None);
        light.first = light.first.min(environment.first);
        light.last = light.last.max(environment.last);
        let mut lifetimes: Vec<_> = graph
            .lifetimes
            .iter()
            .filter(|life| {
                let id = index(&life.resource);
                id != directional && id != self.environment
            })
            .cloned()
            .collect();
        let insertion = lifetimes.partition_point(|life| {
            (life.first, index(&life.resource)) < (light.first, directional)
        });
        lifetimes.insert(insertion, light);
        lifetimes
    }

    /// Rebase debug-formatted resource indices by removing the extra declaration.
    /// All other snapshot bytes are copied verbatim.
    /// # Panics
    /// Panics for malformed indices or a remaining reference to the extra buffer.
    pub fn resource_indices(&self, record: &str) -> String {
        without_resource_index(record, self.environment)
    }
}

fn without_resource_index(record: &str, removed: usize) -> String {
    let mut parts = record.split(RESOURCE);
    let mut historical = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let (id, rest) = part.split_once(',').expect("resource index separator");
        let id: usize = id.parse().expect("graph resource index");
        assert_ne!(id, removed);
        let historical_id = id - usize::from(id > removed);
        write!(historical, "{RESOURCE}{historical_id},{rest}").expect("snapshot string");
    }
    historical
}

/// Projection for graph fixtures captured before the retained chart inputs.
#[cfg(feature = "render-registry")]
pub struct BeforeChartSnapshot<'a> {
    resource: Option<&'a ResourceId>,
}

#[cfg(feature = "render-registry")]
impl<'a> BeforeChartSnapshot<'a> {
    /// Find the retained chart inputs by graph ownership.
    pub fn new(graph: &'a CompiledGraph) -> Self {
        Self {
            resource: thrum_render::registry_audit::atmosphere_chart_snapshot(graph),
        }
    }

    /// Exclude just the new uniform lifetime from a historical comparison.
    pub fn lifetimes(&self, lifetimes: Vec<Lifetime>) -> Vec<Lifetime> {
        lifetimes
            .into_iter()
            .filter(|life| self.resource != Some(&life.resource))
            .collect()
    }

    /// Rebase resource indices after the retained input buffer.
    pub fn resource_indices(&self, record: &str) -> String {
        self.resource.map_or_else(
            || record.to_owned(),
            |resource| without_resource_index(record, index(resource)),
        )
    }

    /// Number of retained chart input buffers excluded from the comparison.
    pub fn buffer_count(&self) -> usize {
        usize::from(self.resource.is_some())
    }
}
