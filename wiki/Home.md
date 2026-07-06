# Chronos Wiki — Home

Welcome to the **Chronos** project wiki!

Chronos is a minimalist, local-first, privacy-respecting work time tracker inspired by the classic utility *AllNetic Working Time Tracker*. It is designed for developers, freelancers, and professionals who need accurate billing reports without invasive activity-tracking spyware.

## 🎯 Project Pillars
* **Absolute User Control:** No silent or forced tracking. The user maintains complete control over start, stop, and pause actions.
* **Granular Task Trees:** Support for infinite hierarchies (Projects > Tasks > Subtasks) with automatic cumulative time summaries.
* **System Integration:** Runs unobtrusively in the background via a system tray icon, supporting quick actions and minimizing clutter.
* **Financial Accuracy:** Native support for categorizing hours as billable (`payable`) vs. non-billable to simplify invoicing.

---

## 🛠️ Architecture & Tech Stack
Chronos is built on a highly efficient and modern Rust stack:
* **Language:** Rust (v1.92+) for performance, safety, and a single lightweight binary footprint.
* **GUI Engine:** `eframe` (egui) for immediate-mode layout rendering.
* **Database:** SQLite via `rusqlite` for fast, lightweight, single-file local storage.
* **Integration:** `tray-icon` for notification area support, and `notify-rust` for desktop alerts.

---

## 📖 Wiki Pages
* [[Home]] — This page.
* [[Architecture]] — Technical layout, schemas, and state engine details.
* [[Local-Deployment]] — How to set up, build, test, and run Chronos locally.
