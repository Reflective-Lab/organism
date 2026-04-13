# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Debate loop, pack profiles, multi-dimension resolver
- Intent packet types with ForbiddenAction, ExpiryAction, reversibility
- Admission control types (AdmissionController trait, 4 feasibility dimensions)
- Intent decomposition (IntentNode tree with authority narrowing)
- Plan annotations (impact, cost, risk modeling)
- Huddle scaffold (HuddleParticipant/Reasoner traits, 6 reasoning systems)
- Adversarial types (Challenge, 5 SkepticismKinds, Skeptic trait)
- Simulation swarm types (5 dimensions, SimulationRunner trait)
- Learning episode types (prediction error, lesson, prior calibration)
- Knowledge lifecycle pack (Signal → Hypothesis → Experiment → Decision → Canonical)
- 13 organizational packs + 8 blueprints
- Billing module (Stripe ACP types, from converge-runtime)
- Vision module (Claude, GPT-4o, Gemini, Pixtral)
- OCR module (photos, screenshots ingestion)

### Changed
- Restructured as organizational intelligence runtime (9 crates)

## 2026-04-06

### Added
- Intelligence crate, kb, and workflow
- Initial organism workspace
