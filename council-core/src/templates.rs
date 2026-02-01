//! Built-in event templates for the galactic exploration simulation.

use crate::event::{Event, EventTemplate, Outcome, ResponseOption, RngCore};
use crate::galaxy::{
    Discovery, GalaxyState, Relation, Sector, SectorType, Species, StateChange, Threat,
};

/// Names for procedurally generated content.
mod names {
    pub const SECTOR_PREFIXES: &[&str] = &[
        "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Theta", "Omega", "Nova", "Sigma",
    ];
    pub const SECTOR_SUFFIXES: &[&str] = &[
        "Quadrant", "Nebula", "Cluster", "Expanse", "Reach", "Void", "Drift", "Sector",
    ];
    pub const SPECIES_PREFIXES: &[&str] = &[
        "Zor", "Krel", "Xan", "Vel", "Mur", "Thal", "Qor", "Nex", "Pax", "Dra",
    ];
    pub const SPECIES_SUFFIXES: &[&str] = &[
        "ians", "oids", "ax", "uri", "eni", "oni", "ari", "eki", "oth", "ix",
    ];
    pub const THREAT_NAMES: &[&str] = &[
        "Space Pirates",
        "Void Swarm",
        "Rogue AI Fleet",
        "Cosmic Storm",
        "Hostile Probes",
        "Dark Matter Entity",
        "Quantum Anomaly",
        "Stellar Plague",
    ];
    pub const DISCOVERY_TYPES: &[&str] = &[
        "Ancient Archive",
        "Power Crystal",
        "Navigation Chart",
        "Shield Technology",
        "Propulsion Upgrade",
        "Communication Array",
        "Medical Breakthrough",
        "Weapons System",
    ];
}

fn random_sector_name(rng: &mut dyn RngCore) -> String {
    let prefix = names::SECTOR_PREFIXES[rng.next_u32() as usize % names::SECTOR_PREFIXES.len()];
    let suffix = names::SECTOR_SUFFIXES[rng.next_u32() as usize % names::SECTOR_SUFFIXES.len()];
    format!("{} {}", prefix, suffix)
}

fn random_species_name(rng: &mut dyn RngCore) -> String {
    let prefix = names::SPECIES_PREFIXES[rng.next_u32() as usize % names::SPECIES_PREFIXES.len()];
    let suffix = names::SPECIES_SUFFIXES[rng.next_u32() as usize % names::SPECIES_SUFFIXES.len()];
    format!("{}{}", prefix, suffix)
}

// ============================================================================
// Exploration Templates
// ============================================================================

/// Detect a signal from an unexplored region.
pub struct UnknownSignalTemplate;

impl EventTemplate for UnknownSignalTemplate {
    fn name(&self) -> &'static str {
        "Unknown Signal"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        galaxy.explored_sectors.len() < 10
    }

    fn generate(&self, _galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let sector_name = random_sector_name(rng);
        let sector_type = match rng.next_u32() % 4 {
            0 => SectorType::Nebula,
            1 => SectorType::AsteroidField,
            2 => SectorType::Habitable,
            _ => SectorType::Void,
        };

        Event {
            description: format!(
                "Long-range sensors detect an unusual signal emanating from an unexplored \
                region. Analysis suggests it originates from the {}.",
                sector_name
            ),
            relevant_expertise: vec![
                ("science".to_string(), 0.4),
                ("exploration".to_string(), 0.4),
                ("engineering".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Dispatch a crewed expedition to investigate".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "The expedition successfully charts the {} and returns with valuable data.",
                            sector_name
                        ),
                        score_delta: 15,
                        state_changes: vec![StateChange::AddSector(Sector {
                            name: sector_name.clone(),
                            sector_type,
                        })],
                    },
                },
                ResponseOption {
                    description: "Send an unmanned probe first".to_string(),
                    outcome: Outcome {
                        description: "The probe returns preliminary data. The region is noted for future exploration.".to_string(),
                        score_delta: 5,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Log the signal but focus on known priorities".to_string(),
                    outcome: Outcome {
                        description: "The signal is archived. Perhaps another time.".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
            ],
        }
    }
}

/// Encounter an anomaly in space.
pub struct AnomalyTemplate;

impl EventTemplate for AnomalyTemplate {
    fn name(&self) -> &'static str {
        "Spatial Anomaly"
    }

    fn is_applicable(&self, _galaxy: &GalaxyState) -> bool {
        true
    }

    fn weight(&self) -> u32 {
        8
    }

    fn generate(&self, _galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        Event {
            description: "A spatial anomaly has been detected nearby. It appears to be \
                a stable wormhole or dimensional rift. Energy readings are off the charts."
                .to_string(),
            relevant_expertise: vec![
                ("science".to_string(), 0.5),
                ("engineering".to_string(), 0.3),
                ("exploration".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Send a research team to study it closely".to_string(),
                    outcome: if rng.next_u32().is_multiple_of(3) {
                        Outcome {
                            description: "The research team makes a breakthrough discovery about spatial physics!".to_string(),
                            score_delta: 20,
                            state_changes: vec![StateChange::AddDiscovery(Discovery {
                                name: "Spatial Dynamics Theory".to_string(),
                                category: "science".to_string(),
                            })],
                        }
                    } else {
                        Outcome {
                            description: "The team gathers useful data, though the anomaly remains mysterious.".to_string(),
                            score_delta: 8,
                            state_changes: vec![],
                        }
                    },
                },
                ResponseOption {
                    description: "Observe from a safe distance with long-range sensors".to_string(),
                    outcome: Outcome {
                        description: "Remote observations provide some data. Playing it safe."
                            .to_string(),
                        score_delta: 3,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Mark as hazardous and establish exclusion zone".to_string(),
                    outcome: Outcome {
                        description: "The anomaly is marked on charts as a navigation hazard."
                            .to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
            ],
        }
    }
}

// ============================================================================
// Contact Templates
// ============================================================================

/// First contact with a new species.
pub struct FirstContactTemplate;

impl EventTemplate for FirstContactTemplate {
    fn name(&self) -> &'static str {
        "First Contact"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        galaxy.known_species.len() < 5
    }

    fn weight(&self) -> u32 {
        12
    }

    fn generate(&self, _galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let species_name = random_species_name(rng);
        let traits = match rng.next_u32() % 3 {
            0 => vec!["curious".to_string(), "peaceful".to_string()],
            1 => vec!["cautious".to_string(), "territorial".to_string()],
            _ => vec!["aggressive".to_string(), "expansionist".to_string()],
        };
        let is_hostile = traits.contains(&"aggressive".to_string());

        Event {
            description: format!(
                "Our explorers have encountered the {}, a previously unknown spacefaring \
                species. Initial observations suggest they are {}.",
                species_name,
                traits.join(" and ")
            ),
            relevant_expertise: vec![
                ("diplomacy".to_string(), 0.5),
                ("culture".to_string(), 0.3),
                ("linguistics".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Initiate peaceful diplomatic contact".to_string(),
                    outcome: if is_hostile {
                        Outcome {
                            description: format!(
                                "The {} view our overtures as weakness and become hostile.",
                                species_name
                            ),
                            score_delta: -10,
                            state_changes: vec![
                                StateChange::AddSpecies(Species {
                                    name: species_name.clone(),
                                    traits: traits.clone(),
                                }),
                                StateChange::SetRelation {
                                    species: species_name.clone(),
                                    relation: Relation::Hostile,
                                },
                            ],
                        }
                    } else {
                        Outcome {
                            description: format!(
                                "The {} respond positively. A new friendship begins!",
                                species_name
                            ),
                            score_delta: 15,
                            state_changes: vec![
                                StateChange::AddSpecies(Species {
                                    name: species_name.clone(),
                                    traits: traits.clone(),
                                }),
                                StateChange::SetRelation {
                                    species: species_name.clone(),
                                    relation: Relation::Friendly,
                                },
                            ],
                        }
                    },
                },
                ResponseOption {
                    description: "Maintain cautious observation before contact".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "We observe the {} from afar, learning about them before deciding on contact.",
                            species_name
                        ),
                        score_delta: 5,
                        state_changes: vec![StateChange::AddSpecies(Species {
                            name: species_name.clone(),
                            traits,
                        })],
                    },
                },
                ResponseOption {
                    description: "Withdraw and avoid contact for now".to_string(),
                    outcome: Outcome {
                        description: "We retreat quietly. The species remains unaware of us.".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
            ],
        }
    }
}

// ============================================================================
// Crisis Templates
// ============================================================================

/// A new threat emerges.
pub struct ThreatEmergenceTemplate;

impl EventTemplate for ThreatEmergenceTemplate {
    fn name(&self) -> &'static str {
        "Threat Emergence"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        galaxy.threats.len() < 3
    }

    fn weight(&self) -> u32 {
        6
    }

    fn generate(&self, _galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let threat_name =
            names::THREAT_NAMES[rng.next_u32() as usize % names::THREAT_NAMES.len()].to_string();
        let severity = (rng.next_u32() % 3) + 1;

        Event {
            description: format!(
                "Alert! {} have been detected approaching our territory. \
                Threat assessment: severity level {}.",
                threat_name, severity
            ),
            relevant_expertise: vec![
                ("military".to_string(), 0.5),
                ("strategy".to_string(), 0.3),
                ("engineering".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Confront the threat with immediate military response".to_string(),
                    outcome: if rng.next_u32().is_multiple_of(2) {
                        Outcome {
                            description: format!("Our forces engage the {}. After a fierce battle, the threat is neutralized!", threat_name),
                            score_delta: 12,
                            state_changes: vec![],
                        }
                    } else {
                        Outcome {
                            description: format!("Our forces engage but cannot fully repel the {}. The threat persists.", threat_name),
                            score_delta: -5,
                            state_changes: vec![StateChange::AddThreat(Threat {
                                name: threat_name.clone(),
                                severity: severity / 2 + 1,
                                rounds_active: 0,
                            })],
                        }
                    },
                },
                ResponseOption {
                    description: "Fortify defenses and prepare for siege".to_string(),
                    outcome: Outcome {
                        description: format!("We strengthen our defenses. The {} probe our perimeter but find no weakness.", threat_name),
                        score_delta: 3,
                        state_changes: vec![StateChange::AddThreat(Threat {
                            name: threat_name.clone(),
                            severity,
                            rounds_active: 0,
                        })],
                    },
                },
                ResponseOption {
                    description: "Attempt diplomatic resolution".to_string(),
                    outcome: Outcome {
                        description: format!("Negotiations with the {} fail. They attack while our guard is down!", threat_name),
                        score_delta: -15,
                        state_changes: vec![StateChange::AddThreat(Threat {
                            name: threat_name,
                            severity: severity + 1,
                            rounds_active: 0,
                        })],
                    },
                },
            ],
        }
    }
}

// ============================================================================
// Discovery Templates
// ============================================================================

/// Find a valuable artifact or technology.
pub struct ArtifactTemplate;

impl EventTemplate for ArtifactTemplate {
    fn name(&self) -> &'static str {
        "Artifact Discovery"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        galaxy.explored_sectors.len() > 1
    }

    fn weight(&self) -> u32 {
        7
    }

    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let sector_idx = rng.next_u32() as usize % galaxy.explored_sectors.len().max(1);
        let sector = galaxy
            .explored_sectors
            .get(sector_idx)
            .map(|s| s.name.as_str())
            .unwrap_or("Home Sector");
        let artifact_name =
            names::DISCOVERY_TYPES[rng.next_u32() as usize % names::DISCOVERY_TYPES.len()];

        Event {
            description: format!(
                "Survey teams in {} have discovered what appears to be \
                an ancient {}. Initial scans suggest it may still be functional.",
                sector, artifact_name
            ),
            relevant_expertise: vec![
                ("archaeology".to_string(), 0.4),
                ("science".to_string(), 0.3),
                ("engineering".to_string(), 0.3),
            ],
            options: vec![
                ResponseOption {
                    description: "Attempt to activate the artifact immediately".to_string(),
                    outcome: if rng.next_u32().is_multiple_of(4) {
                        Outcome {
                            description: format!(
                                "The {} activates but overloads, causing damage before failing.",
                                artifact_name
                            ),
                            score_delta: -10,
                            state_changes: vec![],
                        }
                    } else {
                        Outcome {
                            description: format!("The {} activates successfully! Its knowledge is integrated into our systems.", artifact_name),
                            score_delta: 18,
                            state_changes: vec![StateChange::AddDiscovery(Discovery {
                                name: artifact_name.to_string(),
                                category: "artifact".to_string(),
                            })],
                        }
                    },
                },
                ResponseOption {
                    description: "Carefully study it before attempting activation".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "Careful analysis reveals the {}'s secrets safely.",
                            artifact_name
                        ),
                        score_delta: 10,
                        state_changes: vec![StateChange::AddDiscovery(Discovery {
                            name: artifact_name.to_string(),
                            category: "artifact".to_string(),
                        })],
                    },
                },
                ResponseOption {
                    description: "Secure the site for later investigation".to_string(),
                    outcome: Outcome {
                        description:
                            "The artifact is secured. We'll return to it when resources allow."
                                .to_string(),
                        score_delta: 2,
                        state_changes: vec![],
                    },
                },
            ],
        }
    }
}

/// Collect all built-in templates.
pub fn default_templates() -> Vec<Box<dyn EventTemplate>> {
    vec![
        Box::new(UnknownSignalTemplate),
        Box::new(AnomalyTemplate),
        Box::new(FirstContactTemplate),
        Box::new(ThreatEmergenceTemplate),
        Box::new(ArtifactTemplate),
    ]
}

/// Select and generate an event from applicable templates.
pub fn generate_event(
    templates: &[Box<dyn EventTemplate>],
    galaxy: &GalaxyState,
    rng: &mut dyn RngCore,
) -> Event {
    let applicable: Vec<_> = templates
        .iter()
        .filter(|t| t.is_applicable(galaxy))
        .collect();

    if applicable.is_empty() {
        // Fallback event
        return Event {
            description: "A quiet period in the cosmos. The council convenes for routine matters."
                .to_string(),
            relevant_expertise: vec![],
            options: vec![ResponseOption {
                description: "Continue as normal".to_string(),
                outcome: Outcome {
                    description: "Business as usual.".to_string(),
                    score_delta: 1,
                    state_changes: vec![],
                },
            }],
        };
    }

    // Weight-based selection
    let total_weight: u32 = applicable.iter().map(|t| t.weight()).sum();
    let mut roll = rng.next_u32() % total_weight;

    for template in &applicable {
        if roll < template.weight() {
            return template.generate(galaxy, rng);
        }
        roll -= template.weight();
    }

    // Fallback (shouldn't happen)
    applicable[0].generate(galaxy, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn unknown_signal_generates_valid_event() {
        let template = UnknownSignalTemplate;
        let galaxy = GalaxyState::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let event = template.generate(&galaxy, &mut rng);
        assert!(!event.description.is_empty());
        assert_eq!(event.options.len(), 3);
        assert!(!event.relevant_expertise.is_empty());
    }

    #[test]
    fn first_contact_generates_species() {
        let template = FirstContactTemplate;
        let galaxy = GalaxyState::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let event = template.generate(&galaxy, &mut rng);
        // At least the diplomatic option should add a species
        let has_species_change = event.options.iter().any(|opt| {
            opt.outcome
                .state_changes
                .iter()
                .any(|c| matches!(c, StateChange::AddSpecies(_)))
        });
        assert!(has_species_change);
    }

    #[test]
    fn generate_event_picks_from_templates() {
        let templates = default_templates();
        let galaxy = GalaxyState::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let event = generate_event(&templates, &galaxy, &mut rng);
        assert!(!event.description.is_empty());
        assert!(!event.options.is_empty());
    }

    #[test]
    fn threat_template_respects_limit() {
        let template = ThreatEmergenceTemplate;
        let mut galaxy = GalaxyState::new();

        assert!(template.is_applicable(&galaxy));

        // Add 3 threats
        for i in 0..3 {
            galaxy.threats.push(Threat {
                name: format!("Threat {}", i),
                severity: 1,
                rounds_active: 0,
            });
        }

        assert!(!template.is_applicable(&galaxy));
    }
}
