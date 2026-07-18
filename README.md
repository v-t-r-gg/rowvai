# Rowva

**Local-first relational spreadsheet CRM**

Rowva is a desktop application for building structured, relational data with the power of a spreadsheet. It combines the flexibility of formulas with proper relational references (inspired by Grist and Airtable), all stored in a single portable `.rowva` file (SQLite).

## Vision

- **Local-first**: Your data lives in one file you control. No accounts, no cloud required.
- **Relational + Formulas**: Link records across tables and use real spreadsheet formulas (300+ Excel-compatible functions via the Formualizer engine).
- **Spreadsheet UX for structured data**: A familiar grid that can handle both flat tables and connected data.

## Current Status

Early prototype. The foundation is in place:

- Create `.rowva` workbooks (SQLite)
- Create tables and columns
- View data in a responsive grid (TanStack Table)

**Not yet implemented** (planned):
- Inserting / editing rows
- Formula columns and recalculation
- `Ref` / `RefList` relational columns
- Opening existing workbooks
- Full persistence round-tripping

See the development plan (internal) for the phased roadmap.

## Tech Stack

**Frontend**
- React 19 + TypeScript + Vite
- TanStack Table
- Tailwind CSS v4

**Backend**
- Tauri 2 (Rust)
- rusqlite (embedded SQLite with WAL)
- formualizer-workbook (powerful formula + dependency graph engine)
- petgraph (for future relational rollups)

## Getting Started

### Prerequisites

- Node.js 18+
- Rust (latest stable) + Cargo
- Tauri prerequisites for your platform: https://tauri.app/start/prerequisites/

### Install & Run (Development)

```bash
# Install JS dependencies
npm install

# Run the Tauri dev server (starts Vite + Rust backend)
npm run tauri dev
```

The app will launch with a clean dark interface. Create a new workbook to begin.

### Build for Production

```bash
npm run tauri build
```

The bundled app will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
src/                  # React frontend
  components/Grid.tsx
  App.tsx
  lib/tauri.ts        # Thin wrapper around Tauri invoke commands

src-tauri/
  src/
    commands.rs       # Tauri commands (new_workbook, create_table, ...)
    storage.rs        # SQLite schema + CRUD helpers
    engine/           # Domain models (Table, Column, RowvaWorkbook)
    lib.rs
  tauri.conf.json
  Cargo.toml
```

The single source of truth for any workbook is the `.rowva` SQLite file written to the app data directory.

## License

Dual-licensed under MIT or Apache-2.0 at your option (same as the Rust dependencies).

---

**Note for contributors**: This project is in active early development. Large architectural decisions (especially around how the formula engine maps to relational tables) are still being finalized. Check recent commits or open an issue before starting big work.
