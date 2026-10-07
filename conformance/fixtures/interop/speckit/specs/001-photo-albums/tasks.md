---

description: "Task list for Photo Albums"
---

# Tasks: Photo Albums

**Input**: Design documents from `/specs/001-photo-albums/`

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 Create project structure per implementation plan
- [ ] T002 [P] Configure linting and formatting tools

---

## Phase 3: User Story 1 - Create an album (Priority: P1) 🎯 MVP

- [ ] T003 [P] [US1] Create the Album model in src/models/album.py
- [ ] T004 [US1] Implement AlbumService in src/services/album.py for FR-001 (depends on T003)

---

## Phase 4: User Story 2 - Reorder albums (Priority: P2)

- [ ] T005 [US2] Implement drag-and-drop reordering in src/ui/albums.py (depends on T004)
