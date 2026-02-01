//! Galaxy state tracking for the exploration simulation.

use std::collections::HashMap;

/// The full state of the galaxy, modified by council decisions.
#[derive(Debug, Clone, Default)]
pub struct GalaxyState {
    /// Current simulation round.
    pub round: u32,
    /// Known regions/sectors of space.
    pub explored_sectors: Vec<Sector>,
    /// Species the council has encountered.
    pub known_species: Vec<Species>,
    /// Diplomatic standings with known species (keyed by species name).
    pub relations: HashMap<String, Relation>,
    /// Technologies and artifacts discovered.
    pub discoveries: Vec<Discovery>,
    /// Active threats facing the council.
    pub threats: Vec<Threat>,
}

impl GalaxyState {
    /// Create a new galaxy state with initial conditions.
    pub fn new() -> Self {
        Self {
            round: 0,
            explored_sectors: vec![Sector {
                name: "Home Sector".to_string(),
                sector_type: SectorType::Habitable,
            }],
            known_species: Vec::new(),
            relations: HashMap::new(),
            discoveries: Vec::new(),
            threats: Vec::new(),
        }
    }

    /// Apply a list of state changes from an event outcome.
    pub fn apply_changes(&mut self, changes: &[StateChange]) {
        for change in changes {
            match change {
                StateChange::AddSector(sector) => {
                    if !self.explored_sectors.iter().any(|s| s.name == sector.name) {
                        self.explored_sectors.push(sector.clone());
                    }
                }
                StateChange::AddSpecies(species) => {
                    if !self.known_species.iter().any(|s| s.name == species.name) {
                        self.known_species.push(species.clone());
                        self.relations
                            .insert(species.name.clone(), Relation::Unknown);
                    }
                }
                StateChange::SetRelation { species, relation } => {
                    self.relations.insert(species.clone(), *relation);
                }
                StateChange::AddDiscovery(discovery) => {
                    self.discoveries.push(discovery.clone());
                }
                StateChange::AddThreat(threat) => {
                    if !self.threats.iter().any(|t| t.name == threat.name) {
                        self.threats.push(threat.clone());
                    }
                }
                StateChange::RemoveThreat(name) => {
                    self.threats.retain(|t| &t.name != name);
                }
                StateChange::ModifyThreatSeverity { name, delta } => {
                    if let Some(threat) = self.threats.iter_mut().find(|t| &t.name == name) {
                        threat.severity = (threat.severity as i32 + delta).max(0) as u32;
                        if threat.severity == 0 {
                            self.threats.retain(|t| &t.name != name);
                        }
                    }
                }
            }
        }
    }

    /// Process ongoing threats, returning score penalty.
    pub fn process_threats(&mut self) -> i32 {
        let mut penalty = 0i32;
        for threat in &mut self.threats {
            threat.rounds_active += 1;
            penalty -= (threat.severity * 3) as i32;
        }
        penalty
    }

    /// Count allied species.
    pub fn allied_count(&self) -> usize {
        self.relations
            .values()
            .filter(|r| matches!(r, Relation::Allied))
            .count()
    }

    /// Count hostile species.
    pub fn hostile_count(&self) -> usize {
        self.relations
            .values()
            .filter(|r| matches!(r, Relation::Hostile))
            .count()
    }
}

/// A region of space that has been explored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    pub name: String,
    pub sector_type: SectorType,
}

/// Types of space sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorType {
    Habitable,
    AsteroidField,
    Nebula,
    Void,
    Anomaly,
}

/// An alien species encountered by the council.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub name: String,
    pub traits: Vec<String>,
}

/// Diplomatic relation with a species.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Unknown,
    Hostile,
    Wary,
    Neutral,
    Friendly,
    Allied,
}

/// A technology or artifact discovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    pub name: String,
    pub category: String,
}

/// An active threat facing the council.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threat {
    pub name: String,
    pub severity: u32,
    pub rounds_active: u32,
}

/// Changes that can be applied to galaxy state.
#[derive(Debug, Clone)]
pub enum StateChange {
    AddSector(Sector),
    AddSpecies(Species),
    SetRelation { species: String, relation: Relation },
    AddDiscovery(Discovery),
    AddThreat(Threat),
    RemoveThreat(String),
    ModifyThreatSeverity { name: String, delta: i32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_galaxy_has_home_sector() {
        let galaxy = GalaxyState::new();
        assert_eq!(galaxy.explored_sectors.len(), 1);
        assert_eq!(galaxy.explored_sectors[0].name, "Home Sector");
    }

    #[test]
    fn apply_add_sector() {
        let mut galaxy = GalaxyState::new();
        let sector = Sector {
            name: "Alpha Quadrant".to_string(),
            sector_type: SectorType::Nebula,
        };
        galaxy.apply_changes(&[StateChange::AddSector(sector)]);
        assert_eq!(galaxy.explored_sectors.len(), 2);
    }

    #[test]
    fn apply_add_species_sets_unknown_relation() {
        let mut galaxy = GalaxyState::new();
        let species = Species {
            name: "Zorblax".to_string(),
            traits: vec!["curious".to_string()],
        };
        galaxy.apply_changes(&[StateChange::AddSpecies(species)]);
        assert_eq!(galaxy.known_species.len(), 1);
        assert_eq!(galaxy.relations.get("Zorblax"), Some(&Relation::Unknown));
    }

    #[test]
    fn threat_processing_applies_penalty() {
        let mut galaxy = GalaxyState::new();
        galaxy.threats.push(Threat {
            name: "Space Pirates".to_string(),
            severity: 2,
            rounds_active: 0,
        });
        let penalty = galaxy.process_threats();
        assert_eq!(penalty, -6); // severity 2 * 3
        assert_eq!(galaxy.threats[0].rounds_active, 1);
    }

    #[test]
    fn remove_threat_when_severity_zero() {
        let mut galaxy = GalaxyState::new();
        galaxy.threats.push(Threat {
            name: "Minor Issue".to_string(),
            severity: 1,
            rounds_active: 0,
        });
        galaxy.apply_changes(&[StateChange::ModifyThreatSeverity {
            name: "Minor Issue".to_string(),
            delta: -1,
        }]);
        assert!(galaxy.threats.is_empty());
    }
}
