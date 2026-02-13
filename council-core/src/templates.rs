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
    pub const RESEARCH_DISCOVERIES: &[&str] = &[
        "Quantum Entanglement Drive",
        "Subspace Field Theory",
        "Graviton Lens Array",
        "Chrono-Spatial Mapping",
        "Plasma Containment Matrix",
        "Bio-Neural Computing",
        "Dark Energy Harvesting",
        "Dimensional Fold Navigation",
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

/// Improve a relation by one step (Unknown -> Wary -> Neutral -> Friendly -> Allied).
fn improve_relation(current: Relation) -> Relation {
    match current {
        Relation::Hostile => Relation::Wary,
        Relation::Unknown | Relation::Wary => Relation::Neutral,
        Relation::Neutral => Relation::Friendly,
        Relation::Friendly | Relation::Allied => Relation::Allied,
    }
}

/// Degrade a relation by one step (Allied -> Friendly -> Neutral -> Wary -> Hostile).
fn degrade_relation(current: Relation) -> Relation {
    match current {
        Relation::Allied => Relation::Friendly,
        Relation::Friendly => Relation::Neutral,
        Relation::Neutral => Relation::Wary,
        Relation::Wary | Relation::Unknown => Relation::Hostile,
        Relation::Hostile => Relation::Hostile,
    }
}

/// Improve a relation by two steps.
fn greatly_improve_relation(current: Relation) -> Relation {
    improve_relation(improve_relation(current))
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

/// Discover a derelict vessel drifting through a known sector.
pub struct DerelictTemplate;

impl EventTemplate for DerelictTemplate {
    fn name(&self) -> &'static str {
        "Derelict Vessel"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        // We need at least one explored sector to plausibly stumble upon wreckage.
        !galaxy.explored_sectors.is_empty()
    }

    fn weight(&self) -> u32 {
        6
    }

    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let sector =
            &galaxy.explored_sectors[rng.next_u32() as usize % galaxy.explored_sectors.len()];
        let discovery =
            names::DISCOVERY_TYPES[rng.next_u32() as usize % names::DISCOVERY_TYPES.len()];
        let threat = names::THREAT_NAMES[rng.next_u32() as usize % names::THREAT_NAMES.len()];

        let risky_salvage = rng.next_u32().is_multiple_of(5);

        Event {
            description: format!(
                "Scanners pick up a derelict vessel drifting within the {}. Its hull markings don’t match any known registry.",
                sector.name
            ),
            relevant_expertise: vec![
                ("exploration".to_string(), 0.35),
                ("engineering".to_string(), 0.35),
                ("science".to_string(), 0.2),
                ("security".to_string(), 0.1),
            ],
            options: vec![
                ResponseOption {
                    description: "Board the vessel and salvage anything useful".to_string(),
                    outcome: if risky_salvage {
                        Outcome {
                            description: format!(
                                "The boarding team recovers a {} — but triggers dormant systems. A new threat emerges: {}.",
                                discovery, threat
                            ),
                            score_delta: 6,
                            state_changes: vec![
                                StateChange::AddDiscovery(Discovery {
                                    name: discovery.to_string(),
                                    category: "salvage".to_string(),
                                }),
                                StateChange::AddThreat(Threat {
                                    name: threat.to_string(),
                                    severity: 1 + (rng.next_u32() % 3),
                                    rounds_active: 0,
                                }),
                            ],
                        }
                    } else {
                        Outcome {
                            description: format!(
                                "The salvage operation is a success. The council secures a {} from the wreck.",
                                discovery
                            ),
                            score_delta: 14,
                            state_changes: vec![StateChange::AddDiscovery(Discovery {
                                name: discovery.to_string(),
                                category: "salvage".to_string(),
                            })],
                        }
                    },
                },
                ResponseOption {
                    description: "Scan it remotely and leave it undisturbed".to_string(),
                    outcome: Outcome {
                        description: "Long-range scans yield useful telemetry and material analysis. Low risk, modest gain."
                            .to_string(),
                        score_delta: 6,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Mark the location and move on".to_string(),
                    outcome: Outcome {
                        description: "The derelict is logged for future expeditions. The council stays focused on current priorities."
                            .to_string(),
                        score_delta: 1,
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

/// Supplies are running low and the council must respond.
pub struct ResourceScarcityTemplate;

impl EventTemplate for ResourceScarcityTemplate {
    fn name(&self) -> &'static str {
        "Resource Scarcity"
    }

    fn is_applicable(&self, _galaxy: &GalaxyState) -> bool {
        true
    }

    fn weight(&self) -> u32 {
        5
    }

    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let severity = (rng.next_u32() % 3) + 1;

        let partner = if galaxy.known_species.is_empty() {
            None
        } else {
            Some(&galaxy.known_species[rng.next_u32() as usize % galaxy.known_species.len()].name)
        };

        let (partner_name, current_relation) = match partner {
            Some(name) => (
                Some(name.clone()),
                galaxy
                    .relations
                    .get(name.as_str())
                    .copied()
                    .unwrap_or(Relation::Unknown),
            ),
            None => (None, Relation::Unknown),
        };

        let trade_success = partner_name
            .as_ref()
            .is_some_and(|_| !matches!(current_relation, Relation::Hostile))
            && !rng.next_u32().is_multiple_of(4);

        let discovery = format!("Closed-Loop Recycling v{}", severity);

        Event {
            description: format!(
                "A critical shortage is developing in fuel and critical materials. Internal forecasts rate it severity {}.",
                severity
            ),
            relevant_expertise: vec![
                ("engineering".to_string(), 0.4),
                ("strategy".to_string(), 0.35),
                ("diplomacy".to_string(), 0.25),
            ],
            options: vec![
                ResponseOption {
                    description: "Impose rationing and efficiency measures".to_string(),
                    outcome: Outcome {
                        description: "Consumption drops and reserves stabilize. Nobody loves it, but it works.".to_string(),
                        score_delta: 3,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Seek emergency trade and resupply agreements".to_string(),
                    outcome: match partner_name {
                        None => Outcome {
                            description: "We have no established contacts to trade with. The council must rely on internal measures.".to_string(),
                            score_delta: -2,
                            state_changes: vec![],
                        },
                        Some(species) if trade_success => Outcome {
                            description: format!(
                                "The {} agree to a resupply deal. Relations improve and the crisis eases.",
                                species
                            ),
                            score_delta: 8,
                            state_changes: vec![StateChange::SetRelation {
                                species: species.clone(),
                                relation: improve_relation(current_relation),
                            }],
                        },
                        Some(species) => Outcome {
                            description: format!(
                                "Negotiations with the {} stall. The shortage worsens and trust erodes.",
                                species
                            ),
                            score_delta: -6,
                            state_changes: vec![StateChange::SetRelation {
                                species: species.clone(),
                                relation: degrade_relation(current_relation),
                            }],
                        },
                    },
                },
                ResponseOption {
                    description: "Attempt a rapid engineering breakthrough to replace the missing resources".to_string(),
                    outcome: if rng.next_u32().is_multiple_of(3) {
                        Outcome {
                            description: format!(
                                "A rushed but successful retrofit delivers {}. The supply crunch is largely mitigated.",
                                discovery
                            ),
                            score_delta: 12,
                            state_changes: vec![StateChange::AddDiscovery(Discovery {
                                name: discovery,
                                category: "engineering".to_string(),
                            })],
                        }
                    } else {
                        Outcome {
                            description: "The retrofit program fails and causes cascading shortages. A long-term crisis is now active.".to_string(),
                            score_delta: -10,
                            state_changes: vec![StateChange::AddThreat(Threat {
                                name: "Resource Shortfall".to_string(),
                                severity,
                                rounds_active: 0,
                            })],
                        }
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

// ============================================================================
// Diplomacy Templates
// ============================================================================

/// A known species requests diplomatic engagement.
pub struct DiplomaticRequestTemplate;

impl EventTemplate for DiplomaticRequestTemplate {
    fn name(&self) -> &'static str {
        "Diplomatic Request"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        !galaxy.known_species.is_empty()
    }

    fn weight(&self) -> u32 {
        9
    }

    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let species_idx = rng.next_u32() as usize % galaxy.known_species.len();
        let species_name = &galaxy.known_species[species_idx].name;
        let current_relation = galaxy
            .relations
            .get(species_name)
            .copied()
            .unwrap_or(Relation::Unknown);

        let generous_relation = greatly_improve_relation(current_relation);
        let negotiate_relation = improve_relation(current_relation);
        let decline_relation = degrade_relation(current_relation);

        Event {
            description: format!(
                "The {} have sent an envoy requesting a formal diplomatic summit. \
                They wish to discuss trade agreements and cultural exchange. \
                Current relations are {:?}.",
                species_name, current_relation
            ),
            relevant_expertise: vec![
                ("diplomacy".to_string(), 0.5),
                ("culture".to_string(), 0.3),
                ("strategy".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Accept generously — offer trade and cultural exchange"
                        .to_string(),
                    outcome: Outcome {
                        description: format!(
                            "The {} are delighted by our generosity. Relations improve significantly!",
                            species_name
                        ),
                        score_delta: 12,
                        state_changes: vec![StateChange::SetRelation {
                            species: species_name.clone(),
                            relation: generous_relation,
                        }],
                    },
                },
                ResponseOption {
                    description: "Negotiate cautiously — seek mutual benefit".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "Careful negotiations with the {} yield a modest agreement.",
                            species_name
                        ),
                        score_delta: 5,
                        state_changes: vec![StateChange::SetRelation {
                            species: species_name.clone(),
                            relation: negotiate_relation,
                        }],
                    },
                },
                ResponseOption {
                    description: "Decline the summit — we have other priorities".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "The {} are offended by our refusal. Relations deteriorate.",
                            species_name
                        ),
                        score_delta: -2,
                        state_changes: vec![StateChange::SetRelation {
                            species: species_name.clone(),
                            relation: decline_relation,
                        }],
                    },
                },
            ],
        }
    }
}

/// A known species proposes a cultural exchange program.
pub struct CulturalExchangeTemplate;

impl EventTemplate for CulturalExchangeTemplate {
    fn name(&self) -> &'static str {
        "Cultural Exchange"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        // Cultural exchange only makes sense if we've met someone and we're not openly at war.
        galaxy.known_species.iter().any(|s| {
            !matches!(
                galaxy
                    .relations
                    .get(&s.name)
                    .copied()
                    .unwrap_or(Relation::Unknown),
                Relation::Hostile
            )
        })
    }

    fn weight(&self) -> u32 {
        7
    }

    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        // Pick a non-hostile species if possible; fallback to any known species.
        let candidates: Vec<_> = galaxy
            .known_species
            .iter()
            .filter(|s| {
                !matches!(
                    galaxy
                        .relations
                        .get(&s.name)
                        .copied()
                        .unwrap_or(Relation::Unknown),
                    Relation::Hostile
                )
            })
            .collect();

        let chosen = if candidates.is_empty() {
            &galaxy.known_species[rng.next_u32() as usize % galaxy.known_species.len()]
        } else {
            candidates[rng.next_u32() as usize % candidates.len()]
        };

        let species_name = &chosen.name;
        let current_relation = galaxy
            .relations
            .get(species_name)
            .copied()
            .unwrap_or(Relation::Unknown);

        let full_exchange = improve_relation(current_relation);
        let limited_exchange = current_relation;
        let decline_relation = degrade_relation(current_relation);

        let discovery = format!("{} Cultural Lexicon", species_name);
        let mishap = rng.next_u32().is_multiple_of(6);

        Event {
            description: format!(
                "The {} invite us to a structured cultural exchange: language mapping, art archives, and diplomatic protocol training. Current relations are {:?}.",
                species_name, current_relation
            ),
            relevant_expertise: vec![
                ("culture".to_string(), 0.4),
                ("diplomacy".to_string(), 0.4),
                ("science".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Commit fully — exchange scholars and share archives".to_string(),
                    outcome: if mishap {
                        Outcome {
                            description: "A translation mishap causes offense during the exchange. Relations cool despite useful insights."
                                .to_string(),
                            score_delta: 2,
                            state_changes: vec![
                                StateChange::AddDiscovery(Discovery {
                                    name: discovery.clone(),
                                    category: "culture".to_string(),
                                }),
                                StateChange::SetRelation {
                                    species: species_name.clone(),
                                    relation: degrade_relation(full_exchange),
                                },
                            ],
                        }
                    } else {
                        Outcome {
                            description: format!(
                                "The exchange succeeds. We compile the {} and relations improve.",
                                discovery
                            ),
                            score_delta: 10,
                            state_changes: vec![
                                StateChange::AddDiscovery(Discovery {
                                    name: discovery.clone(),
                                    category: "culture".to_string(),
                                }),
                                StateChange::SetRelation {
                                    species: species_name.clone(),
                                    relation: full_exchange,
                                },
                            ],
                        }
                    },
                },
                ResponseOption {
                    description: "Accept cautiously — run a limited exchange".to_string(),
                    outcome: Outcome {
                        description: "A small exchange program runs smoothly. Incremental trust is built.".to_string(),
                        score_delta: 5,
                        state_changes: vec![StateChange::SetRelation {
                            species: species_name.clone(),
                            relation: limited_exchange,
                        }],
                    },
                },
                ResponseOption {
                    description: "Decline — focus on strategic priorities".to_string(),
                    outcome: Outcome {
                        description: "We politely decline. The relationship suffers from the missed opportunity.".to_string(),
                        score_delta: -1,
                        state_changes: vec![StateChange::SetRelation {
                            species: species_name.clone(),
                            relation: decline_relation,
                        }],
                    },
                },
            ],
        }
    }
}

// ============================================================================
// Research Templates
// ============================================================================

/// A technological breakthrough becomes possible after accumulating discoveries.
pub struct TechBreakthroughTemplate;

impl EventTemplate for TechBreakthroughTemplate {
    fn name(&self) -> &'static str {
        "Tech Breakthrough"
    }

    fn is_applicable(&self, galaxy: &GalaxyState) -> bool {
        galaxy.discoveries.len() >= 3
    }

    fn weight(&self) -> u32 {
        7
    }

    fn generate(&self, _galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event {
        let discovery_name = names::RESEARCH_DISCOVERIES
            [rng.next_u32() as usize % names::RESEARCH_DISCOVERIES.len()];

        Event {
            description: format!(
                "Our scientists report that recent discoveries have opened a path to \
                a major breakthrough: {}. Significant resources would be required to pursue it.",
                discovery_name
            ),
            relevant_expertise: vec![
                ("science".to_string(), 0.5),
                ("engineering".to_string(), 0.3),
                ("exploration".to_string(), 0.2),
            ],
            options: vec![
                ResponseOption {
                    description: "Full investment — redirect all research capacity".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "Massive investment pays off! {} is achieved, revolutionizing our capabilities.",
                            discovery_name
                        ),
                        score_delta: 18,
                        state_changes: vec![StateChange::AddDiscovery(Discovery {
                            name: discovery_name.to_string(),
                            category: "research".to_string(),
                        })],
                    },
                },
                ResponseOption {
                    description: "Methodical research — steady progress over time".to_string(),
                    outcome: Outcome {
                        description: format!(
                            "Patient research yields results. {} is added to our knowledge base.",
                            discovery_name
                        ),
                        score_delta: 8,
                        state_changes: vec![StateChange::AddDiscovery(Discovery {
                            name: discovery_name.to_string(),
                            category: "research".to_string(),
                        })],
                    },
                },
                ResponseOption {
                    description: "Archive the findings for later".to_string(),
                    outcome: Outcome {
                        description: "The research notes are filed away. Perhaps we'll revisit them."
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
        Box::new(DerelictTemplate),
        Box::new(AnomalyTemplate),
        Box::new(FirstContactTemplate),
        Box::new(ThreatEmergenceTemplate),
        Box::new(ResourceScarcityTemplate),
        Box::new(ArtifactTemplate),
        Box::new(DiplomaticRequestTemplate),
        Box::new(CulturalExchangeTemplate),
        Box::new(TechBreakthroughTemplate),
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
    fn derelict_generates_salvage_or_threat() {
        let template = DerelictTemplate;
        let mut galaxy = GalaxyState::new();
        // Ensure at least one non-home sector exists so selection is meaningful.
        galaxy.explored_sectors.push(Sector {
            name: "Beta Expanse".to_string(),
            sector_type: SectorType::Void,
        });
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let event = template.generate(&galaxy, &mut rng);
        assert_eq!(event.options.len(), 3);

        let has_discovery = event.options.iter().any(|opt| {
            opt.outcome
                .state_changes
                .iter()
                .any(|c| matches!(c, StateChange::AddDiscovery(_)))
        });
        assert!(has_discovery);
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

    // ====================================================================
    // Relation helper tests
    // ====================================================================

    #[test]
    fn improve_relation_steps_up() {
        assert_eq!(improve_relation(Relation::Hostile), Relation::Wary);
        assert_eq!(improve_relation(Relation::Unknown), Relation::Neutral);
        assert_eq!(improve_relation(Relation::Wary), Relation::Neutral);
        assert_eq!(improve_relation(Relation::Neutral), Relation::Friendly);
        assert_eq!(improve_relation(Relation::Friendly), Relation::Allied);
        assert_eq!(improve_relation(Relation::Allied), Relation::Allied);
    }

    #[test]
    fn degrade_relation_steps_down() {
        assert_eq!(degrade_relation(Relation::Allied), Relation::Friendly);
        assert_eq!(degrade_relation(Relation::Friendly), Relation::Neutral);
        assert_eq!(degrade_relation(Relation::Neutral), Relation::Wary);
        assert_eq!(degrade_relation(Relation::Wary), Relation::Hostile);
        assert_eq!(degrade_relation(Relation::Unknown), Relation::Hostile);
        assert_eq!(degrade_relation(Relation::Hostile), Relation::Hostile);
    }

    #[test]
    fn greatly_improve_moves_two_steps() {
        assert_eq!(
            greatly_improve_relation(Relation::Hostile),
            Relation::Neutral
        );
        assert_eq!(
            greatly_improve_relation(Relation::Unknown),
            Relation::Friendly
        );
        assert_eq!(
            greatly_improve_relation(Relation::Neutral),
            Relation::Allied
        );
        assert_eq!(
            greatly_improve_relation(Relation::Friendly),
            Relation::Allied
        );
    }

    // ====================================================================
    // DiplomaticRequestTemplate tests
    // ====================================================================

    #[test]
    fn diplomatic_request_applicable_with_species() {
        let template = DiplomaticRequestTemplate;
        let mut galaxy = GalaxyState::new();

        assert!(!template.is_applicable(&galaxy));

        galaxy.known_species.push(Species {
            name: "Zorblax".to_string(),
            traits: vec!["peaceful".to_string()],
        });
        galaxy
            .relations
            .insert("Zorblax".to_string(), Relation::Neutral);

        assert!(template.is_applicable(&galaxy));
    }

    #[test]
    fn diplomatic_request_has_correct_weight() {
        let template = DiplomaticRequestTemplate;
        assert_eq!(template.weight(), 9);
    }

    #[test]
    fn diplomatic_request_generates_three_options_with_set_relation() {
        let template = DiplomaticRequestTemplate;
        let mut galaxy = GalaxyState::new();
        galaxy.known_species.push(Species {
            name: "Xanuri".to_string(),
            traits: vec!["curious".to_string()],
        });
        galaxy
            .relations
            .insert("Xanuri".to_string(), Relation::Neutral);
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);

        let event = template.generate(&galaxy, &mut rng);
        assert_eq!(event.options.len(), 3);

        // Every option should contain a SetRelation state change
        for option in &event.options {
            let has_set_relation = option
                .outcome
                .state_changes
                .iter()
                .any(|c| matches!(c, StateChange::SetRelation { .. }));
            assert!(
                has_set_relation,
                "Option '{}' missing SetRelation change",
                option.description
            );
        }
    }

    // ====================================================================
    // CulturalExchangeTemplate tests
    // ====================================================================

    #[test]
    fn cultural_exchange_applicable_with_non_hostile_species() {
        let template = CulturalExchangeTemplate;
        let mut galaxy = GalaxyState::new();

        assert!(!template.is_applicable(&galaxy));

        galaxy.known_species.push(Species {
            name: "Veloni".to_string(),
            traits: vec!["curious".to_string()],
        });
        galaxy
            .relations
            .insert("Veloni".to_string(), Relation::Neutral);

        assert!(template.is_applicable(&galaxy));

        // If all species are hostile, exchange should not be applicable.
        let mut hostile_only = GalaxyState::new();
        hostile_only.known_species.push(Species {
            name: "Draix".to_string(),
            traits: vec!["aggressive".to_string()],
        });
        hostile_only
            .relations
            .insert("Draix".to_string(), Relation::Hostile);
        assert!(!template.is_applicable(&hostile_only));
    }

    #[test]
    fn cultural_exchange_has_correct_weight() {
        let template = CulturalExchangeTemplate;
        assert_eq!(template.weight(), 7);
    }

    #[test]
    fn cultural_exchange_generates_relation_changes_and_discovery() {
        let template = CulturalExchangeTemplate;
        let mut galaxy = GalaxyState::new();
        galaxy.known_species.push(Species {
            name: "Qoreki".to_string(),
            traits: vec!["peaceful".to_string()],
        });
        galaxy
            .relations
            .insert("Qoreki".to_string(), Relation::Wary);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1234);

        let event = template.generate(&galaxy, &mut rng);
        assert_eq!(event.options.len(), 3);

        for option in &event.options {
            let has_set_relation = option
                .outcome
                .state_changes
                .iter()
                .any(|c| matches!(c, StateChange::SetRelation { .. }));
            assert!(has_set_relation);
        }

        let option0_has_discovery = event.options[0]
            .outcome
            .state_changes
            .iter()
            .any(|c| matches!(c, StateChange::AddDiscovery(_)));
        assert!(option0_has_discovery);
    }

    // ====================================================================
    // ResourceScarcityTemplate tests
    // ====================================================================

    #[test]
    fn resource_scarcity_is_always_applicable() {
        let template = ResourceScarcityTemplate;
        let galaxy = GalaxyState::new();
        assert!(template.is_applicable(&galaxy));
    }

    #[test]
    fn resource_scarcity_has_correct_weight() {
        let template = ResourceScarcityTemplate;
        assert_eq!(template.weight(), 5);
    }

    #[test]
    fn resource_scarcity_generates_three_options_and_last_has_state_change() {
        let template = ResourceScarcityTemplate;
        let galaxy = GalaxyState::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(2026);

        let event = template.generate(&galaxy, &mut rng);
        assert_eq!(event.options.len(), 3);
        assert!(!event.relevant_expertise.is_empty());

        // The engineering option should always either add a discovery or activate a threat.
        let last = &event.options[2].outcome.state_changes;
        assert!(
            last.iter()
                .any(|c| matches!(c, StateChange::AddDiscovery(_)))
                || last.iter().any(|c| matches!(c, StateChange::AddThreat(_)))
        );
    }

    // ====================================================================
    // TechBreakthroughTemplate tests
    // ====================================================================

    #[test]
    fn tech_breakthrough_applicable_with_enough_discoveries() {
        let template = TechBreakthroughTemplate;
        let mut galaxy = GalaxyState::new();

        assert!(!template.is_applicable(&galaxy));

        // Add 2 — still not enough
        for i in 0..2 {
            galaxy.discoveries.push(Discovery {
                name: format!("Discovery {}", i),
                category: "science".to_string(),
            });
        }
        assert!(!template.is_applicable(&galaxy));

        // Add third — now applicable
        galaxy.discoveries.push(Discovery {
            name: "Discovery 2".to_string(),
            category: "science".to_string(),
        });
        assert!(template.is_applicable(&galaxy));
    }

    #[test]
    fn tech_breakthrough_has_correct_weight() {
        let template = TechBreakthroughTemplate;
        assert_eq!(template.weight(), 7);
    }

    #[test]
    fn tech_breakthrough_first_two_options_add_discovery() {
        let template = TechBreakthroughTemplate;
        let mut galaxy = GalaxyState::new();
        for i in 0..3 {
            galaxy.discoveries.push(Discovery {
                name: format!("Discovery {}", i),
                category: "science".to_string(),
            });
        }
        let mut rng = rand::rngs::StdRng::seed_from_u64(77);

        let event = template.generate(&galaxy, &mut rng);
        assert_eq!(event.options.len(), 3);

        // Options 0 and 1 should have AddDiscovery
        for idx in 0..2 {
            let has_discovery = event.options[idx]
                .outcome
                .state_changes
                .iter()
                .any(|c| matches!(c, StateChange::AddDiscovery(_)));
            assert!(has_discovery, "Option {} should add a discovery", idx);
        }

        // Option 2 (archive) should have no state changes
        assert!(
            event.options[2].outcome.state_changes.is_empty(),
            "Archive option should have no state changes"
        );
    }

    #[test]
    fn default_templates_includes_new_templates() {
        let templates = default_templates();
        let names: Vec<&str> = templates.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"Derelict Vessel"));
        assert!(names.contains(&"Resource Scarcity"));
        assert!(names.contains(&"Diplomatic Request"));
        assert!(names.contains(&"Cultural Exchange"));
        assert!(names.contains(&"Tech Breakthrough"));
        assert_eq!(templates.len(), 10);
    }
}
