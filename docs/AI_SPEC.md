# AI Scope Decision

Local AI has been removed from the Household Bills roadmap by project decision.

The application remains fully deterministic and local. No language model runtime, model files, AI sidecar, cloud AI API, AI settings, or AI proposal workflow should be added unless the project owner explicitly reopens that scope in a future version.

All bill scheduling, payment guidance, cash-flow forecasts, savings/debt calculations, warnings, and recommendations are produced by explicit Rust rules and SQLite data.
