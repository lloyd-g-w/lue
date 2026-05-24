pub const APP_CSS: &str = r#"
:root {
  --bg: #f6f7f4;
  --surface: #fbfcfa;
  --surface-raised: #ffffff;
  --surface-muted: #eef1ed;
  --ink: #141312;
  --muted: #626a63;
  --line: #d7ddd6;
  --line-strong: #b5c0b7;
  --accent: #18654e;
  --accent-soft: #dfebe6;
  --success: #2b7447;
  --danger: #b0404c;
  --warning: #8e691d;
  --left: #756d64;
  --shadow: 0 10px 30px rgba(20, 19, 18, 0.05);
  --radius: 16px;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: "Manrope", sans-serif;
  color: var(--ink);
  background: var(--bg);
}

a {
  color: inherit;
  text-decoration: none;
}

.shell {
  max-width: 90rem;
  margin: 0 auto;
  padding: clamp(1rem, 2vw, 1.5rem) clamp(1rem, 2vw, 1.5rem) 4rem;
}

.landing-layout,
.admin-grid,
.workspace-columns,
.queue-page-layout {
  display: grid;
  gap: 24px;
}

.landing-layout {
  grid-template-columns: minmax(0, 1fr) minmax(22rem, 28rem);
  align-items: center;
  min-height: calc(100vh - 120px);
}

.admin-shell {
  display: grid;
  gap: 20px;
}

.admin-grid {
  grid-template-columns: minmax(18rem, 26%) minmax(0, 1fr);
  align-items: start;
}

.workspace-columns {
  grid-template-columns: minmax(22rem, 31%) minmax(0, 1fr);
  gap: 20px;
}

.queue-page-layout {
  width: min(100%, 42rem);
  margin: clamp(1rem, 5vh, 4rem) auto 0;
  grid-template-columns: 1fr;
  gap: 16px;
  align-items: stretch;
}

.login-panel,
.sidebar-panel,
.workspace-panel,
.workspace-header,
.request-list-panel,
.request-detail-panel,
.queue-hero-panel,
.queue-form-panel,
.empty-stage {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 24px;
  box-shadow: var(--shadow);
}

.login-panel {
  display:grid;
  gap: 18px;
}

.login-panel,
.request-detail-panel,
.queue-form-panel {
  background: var(--surface-raised);
}

.queue-hero-panel,
.queue-form-panel {
  background: transparent;
  border: none;
  border-radius: 0;
  padding: 0;
  box-shadow: none;
}

.queue-hero-panel h1 {
  max-width: none;
  font-size: clamp(2rem, 5vw, 3rem);
}

.landing-copy {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 24px;
  min-height: min(34rem, 70vh);
  padding-right: clamp(0rem, 4vw, 3rem);
}

.sidebar-panel,
.workspace-panel,
.request-list-panel,
.request-detail-panel,
.queue-hero-panel,
.queue-form-panel,
.page-panel,
.queue-workspace {
  display: grid;
  gap: 12px;
}

.sidebar-block,
.admin-nav,
.point-list,
.field-list,
.list-shell,
.detail-list,
.action-stack,
.form-stack {
  display: grid;
  gap: 14px;
}

.admin-header,
.detail-header,
.panel-header,
.button-row,
.toggle-row,
.request-row-top,
.request-row-meta,
.list-row-main,
.row-stats,
.action-bar,
.request-meta-strip {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
}

.kicker,
.label,
.ticket-label,
.detail-key {
  margin: 0;
  text-transform: uppercase;
  letter-spacing: 0;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--muted);
}

h1,
h2,
h3,
.page-title {
  margin: 6px 0;
  line-height: 1.08;
  letter-spacing: 0;
}

h1 {
  font-size: clamp(2.5rem, 6vw, 4.2rem);
  max-width: 12ch;
}

h2,
.page-title {
  font-size: clamp(1.7rem, 3vw, 2.5rem);
}

h3 {
  font-size: 1.02rem;
}

.landing-lede,
.lede,
.hint,
.feedback,
.row-meta,
.request-subline {
  margin: 0;
  color: var(--muted);
  line-height: 1.65;
}

.landing-lede {
  font-size: 1.08rem;
  max-width: 42rem;
}

.request-name {
  margin: 0 0 4px;
  font-weight: 700;
  letter-spacing: 0;
}

.point-row {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 14px;
  padding: 16px 0;
  border-top: 1px solid var(--line);
}

.point-row:first-child {
  border-top: none;
  padding-top: 0;
}

.point-badge,
.counter-chip,
.counter-pill,
.badge,
.status-pill {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 7px 11px;
  border: 1px solid var(--line);
  font-size: 0.8rem;
  font-weight: 700;
}

.point-badge,
.counter-chip,
.counter-pill {
  background: var(--surface-muted);
  color: var(--ink);
}

.muted-pill {
  color: var(--muted);
}

.mono,
.small-text,
code {
  font-family: "IBM Plex Mono", monospace;
}

.small-text {
  font-size: 0.8rem;
}

.input-group {
  display: grid;
  gap: 10px;
}

.input,
.button,
.icon-button,
.list-row,
.request-row {
  font: inherit;
}

.input {
  width: 100%;
  padding: 14px 15px;
  border-radius: 16px;
  border: 1px solid var(--line);
  background: var(--surface-raised);
  color: var(--ink);
  outline: none;
  transition: border-color 120ms ease, box-shadow 120ms ease;
}

.input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 4px rgba(24, 101, 78, 0.1);
}

.button,
.icon-button,
.list-row,
.request-row {
  border-radius: 14px;
  border: 1px solid var(--line);
  cursor: pointer;
  transition: transform 120ms ease, border-color 120ms ease, background 120ms ease;
}

.button:hover,
.icon-button:hover,
.list-row:hover,
.request-row:hover {
  transform: translateY(-1px);
}

.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 2.8rem;
  padding: 12px 14px;
  font-weight: 700;
  text-align: center;
}

.button-primary {
  background: var(--ink);
  color: white;
  border-color: transparent;
}

.button-secondary,
.icon-button {
  background: var(--surface-muted);
  color: var(--ink);
}

.button.success {
  background: var(--success);
  color: white;
  border-color: transparent;
}

.button.danger {
  background: var(--danger);
  color: white;
  border-color: transparent;
}

.icon-button {
  padding: 12px 12px;
}

.toggle-row {
  justify-content: flex-start;
}

.access-switch {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px 8px 8px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-raised);
  font-weight: 800;
  cursor: pointer;
  transition: border-color 120ms ease, background 120ms ease;
}

.access-switch:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted);
}

.access-switch input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.switch-track {
  position: relative;
  width: 42px;
  height: 24px;
  border-radius: 999px;
  background: var(--line-strong);
  transition: background 120ms ease;
}

.switch-thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 1px 4px rgba(20, 19, 18, 0.25);
  transition: transform 120ms ease;
}

.access-switch input:checked + .switch-track {
  background: var(--accent);
}

.access-switch input:checked + .switch-track .switch-thumb {
  transform: translateX(18px);
}

.access-switch input:focus-visible + .switch-track {
  box-shadow: 0 0 0 4px rgba(24, 101, 78, 0.12);
}

.switch-label {
  color: var(--ink);
  font-size: 0.9rem;
}

.list-row,
.request-row {
  width: 100%;
  text-align: left;
  background: var(--surface-raised);
  padding: 14px;
}

.list-row-active,
.request-row-active {
  background: #f5f1ea;
  border-color: var(--ink);
}

.detail-list {
  border-top: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
  padding: 8px 0;
}

.compact-list {
  padding: 0;
}

.detail-row {
  display: grid;
  grid-template-columns: minmax(9rem, 24%) minmax(0, 1fr);
  gap: 16px;
  padding: 14px 0;
  border-top: 1px solid var(--line);
}

.detail-row:first-child {
  border-top: none;
}

.detail-value {
  line-height: 1.6;
}

.ticket-panel,
.empty-panel,
.empty-stage {
  background: var(--surface-raised);
  border: 1px solid var(--line);
  border-radius: 18px;
  padding: 18px;
}

.muted-ticket {
  background: var(--surface-muted);
}

.queue-meta-line {
  display: flex;
  gap: 10px;
  align-items: center;
  color: var(--muted);
  font-weight: 700;
}

.queue-status-block {
  display: grid;
  gap: 14px;
  margin-top: 16px;
  padding: 18px 0 0;
  background: transparent;
  border: none;
  border-top: 1px solid var(--line);
  border-radius: 0;
}

.request-list-shell {
  max-height: min(42rem, 70vh);
  overflow: auto;
  padding-right: 4px;
}

.request-list-shell::-webkit-scrollbar {
  width: 10px;
}

.request-list-shell::-webkit-scrollbar-thumb {
  background: #d3cbc0;
  border-radius: 999px;
}

.badge-pending,
.pending-bg {
  background: #f4ecde;
  color: var(--warning);
  border-color: #e4d3ad;
}

.badge-claimed,
.claimed-bg {
  background: var(--accent-soft);
  color: var(--accent);
  border-color: #bfd7cd;
}

.badge-left,
.left-bg {
  background: #ebe6df;
  color: var(--left);
  border-color: #d5ccc1;
}

.badge-resolved,
.resolved-bg {
  background: #e4f1e8;
  color: var(--success);
  border-color: #c2d9c8;
}

.badge-denied,
.denied-bg {
  background: #f8e6e8;
  color: var(--danger);
  border-color: #e6c0c5;
}

.feedback {
  color: var(--danger);
  font-weight: 600;
  margin: 10px 0 0 0;
  padding: 0.9rem 1rem;
  border: 1px solid #e6c0c5;
  border-radius: 14px;
  background: #fff8f8;
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(0.5rem) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.connection-banner {
  position: fixed;
  right: 0.9rem;
  bottom: 0.9rem;
  z-index: 24;
  width: 0.7rem;
  height: 0.7rem;
  padding: 0;
  border: none;
  background: transparent;
}

.connection-banner div {
  display: none;
}

.connection-orb {
  display: block;
  width: 0.7rem;
  height: 0.7rem;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 0 0.25rem rgba(24, 101, 78, 0.08);
}

.connection-live .connection-orb {
  background: var(--success);
}

.connection-connecting .connection-orb,
.connection-reconnecting .connection-orb {
  background: var(--warning);
  animation: pulse-connection 1.4s ease-in-out infinite;
}

.connection-reconnecting .connection-orb {
  background: var(--danger);
}

@keyframes pulse-connection {
  0%,
  100% {
    transform: scale(1);
    opacity: 0.8;
  }
  50% {
    transform: scale(1.18);
    opacity: 1;
  }
}

.floating-feedback {
  position: fixed;
  right: clamp(1rem, 3vw, 2rem);
  bottom: clamp(1rem, 3vw, 2rem);
  z-index: 30;
  width: min(92vw, 28rem);
  box-shadow: 0 18px 50px rgba(20, 19, 18, 0.18);
  animation: toast-in 160ms ease-out;
}

.inspector-note {
  padding-top: 4px;
}

.table-page-section {
  display: grid;
  gap: 18px;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 24px;
  box-shadow: var(--shadow);
}

.page-breadcrumbs {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  color: var(--muted);
}

.breadcrumb-link {
  font: inherit;
  border: none;
  background: transparent;
  color: inherit;
  padding: 0;
  cursor: pointer;
}

.breadcrumb-link:hover {
  color: var(--ink);
}

.table-shell {
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: var(--surface-raised);
}

.archive-stack {
  display: grid;
  gap: 18px;
}

.archive-queue-panel {
  display: grid;
  gap: 14px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: var(--surface-raised);
}

.archive-actions {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.archive-entry-count {
  display: inline-flex;
  align-items: center;
  min-height: 2.35rem;
  padding: 0 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 0.78rem;
  font-weight: 800;
}

.archive-expand-button {
  min-height: 2.35rem;
  padding: 0 13px;
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  background: var(--surface-raised);
  color: var(--ink);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 800;
  cursor: pointer;
  transition: background 120ms ease, border-color 120ms ease;
}

.archive-expand-button:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  min-width: 100%;
}

.data-table th,
.data-table td {
  padding: 16px 18px;
  border-bottom: 1px solid var(--line);
  text-align: left;
  vertical-align: top;
}

.data-table th {
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0;
  color: var(--muted);
  background: #f7f3ec;
}

.data-table tbody tr:last-child td {
  border-bottom: none;
}

.data-table tbody tr:hover {
  background: #faf6ef;
}

.table-primary {
  display: flex;
  gap: 8px;
  align-items: center;
  font-weight: 700;
}

.table-inline-note {
  display: inline-flex;
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: var(--surface-muted);
  font-size: 0.72rem;
  color: var(--muted);
}

.table-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}

.action-button {
  padding: 0.35rem 0.55rem;
  border: 1px solid transparent;
  border-radius: 999px;
  background: transparent;
  color: var(--muted);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 800;
  cursor: pointer;
  transition: color 120ms ease, background 120ms ease, border-color 120ms ease;
}

.action-button:hover {
  color: var(--ink);
  background: var(--surface-muted);
  border-color: var(--line);
}

.action-strong {
  color: var(--accent);
}

.action-success {
  color: var(--success);
}

.action-danger {
  color: var(--danger);
}

.admin-nav {
  margin-top: 4px;
}

.admin-nav-button {
  width: 100%;
  padding: 14px 15px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface-raised);
  color: var(--ink);
  font: inherit;
  font-weight: 800;
  text-align: left;
  cursor: pointer;
  transition: transform 120ms ease, border-color 120ms ease, background 120ms ease;
}

.admin-nav-button:hover {
  transform: translateY(-1px);
  border-color: var(--line-strong);
}

.admin-nav-button-active {
  background: var(--ink);
  color: white;
  border-color: transparent;
}

.split-view-section {
  min-height: min(35rem, 72vh);
  align-content: start;
}

.create-queue-section {
  grid-template-rows: auto 1fr;
  min-height: clamp(34rem, calc(100vh - 14rem), 56rem);
  align-content: stretch;
}

.main-panel {
  min-width: 0;
}

.wide-form {
  width: 100%;
  max-width: 48rem;
}

.create-queue-form {
  max-width: none;
  align-self: stretch;
  align-content: start;
}

.field-editor-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  gap: 10px;
  align-items: center;
}

.field-required-toggle {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  min-height: 2.8rem;
  padding: 0 12px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-raised);
  color: var(--muted);
  font-weight: 800;
  cursor: pointer;
  white-space: nowrap;
}

.field-required-toggle input {
  width: 16px;
  height: 16px;
  accent-color: var(--accent);
}

.account-management-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(18rem, 0.8fr);
  gap: 18px;
  align-items: start;
}

.account-list-panel {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.account-section-header {
  display: flex;
  gap: 14px;
  align-items: start;
  justify-content: space-between;
  flex-wrap: wrap;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--line);
}

.account-section-header h3,
.account-card h4 {
  margin: 0;
}

.account-role-summary,
.account-card-list {
  display: grid;
  gap: 10px;
}

.account-role-summary {
  grid-template-columns: repeat(3, auto);
  align-items: center;
}

.account-role-chip,
.role-pill {
  display: inline-flex;
  align-items: center;
  width: max-content;
  min-height: 2rem;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--muted);
  font-size: 0.76rem;
  font-weight: 800;
  white-space: nowrap;
}

.role-pill {
  background: var(--surface-raised);
  color: var(--ink);
}

.role-super {
  border-color: #d1b766;
  background: #fff7d7;
}

.role-admin {
  border-color: #9fc8b9;
  background: var(--accent-soft);
}

.role-user {
  border-color: var(--line);
  background: var(--surface-muted);
}

.account-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 14px;
  align-items: center;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface-raised);
}

.account-card-main,
.account-card-side {
  display: flex;
  gap: 12px;
  align-items: center;
  min-width: 0;
}

.account-card-side {
  justify-content: flex-end;
  flex-wrap: wrap;
}

.account-avatar {
  display: grid;
  place-items: center;
  flex: 0 0 2.5rem;
  width: 2.5rem;
  height: 2.5rem;
  border-radius: 999px;
  background: var(--ink);
  color: white;
  font-size: 0.82rem;
  font-weight: 900;
}

.compact-account-list .account-card,
.group-card {
  grid-template-columns: 1fr;
  align-items: start;
}

.checkbox-list {
  display: grid;
  gap: 8px;
  max-height: min(18rem, 45vh);
  overflow: auto;
}

.check-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-raised);
}

.share-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(min(16rem, 100%), 1fr));
  gap: 14px;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(20, 19, 18, 0.34);
  backdrop-filter: blur(6px);
}

.modal-panel {
  width: min(92vw, 42rem);
  max-height: min(88vh, 48rem);
  overflow: auto;
  padding: clamp(1rem, 3vw, 1.5rem);
  border: 1px solid var(--line);
  border-radius: 24px;
  background: var(--surface-raised);
  box-shadow: 0 24px 80px rgba(20, 19, 18, 0.22);
}

.modal-member-list {
  max-height: min(24rem, 45vh);
}

.compact-header {
  gap: 8px;
  align-items: start;
}

.compact-header h2 {
  font-size: clamp(1.35rem, 2vw, 1.8rem);
}

.join-panel-grid {
  display: grid;
  gap: 16px;
}

.join-access-block,
.join-form-block {
  display: grid;
  gap: 14px;
  min-width: 0;
}

.join-access-block {
  padding-bottom: 16px;
  border-bottom: 1px solid var(--line);
}

.auth-inline-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
  align-items: stretch;
}

.auth-submit {
  width: 100%;
}

.signed-in-strip {
  display: flex;
  gap: 14px;
  align-items: center;
  justify-content: space-between;
  padding: 14px 0;
  border-top: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
  color: var(--muted);
  font-weight: 700;
}

.locked-block {
  padding: 18px;
  border: 1px dashed var(--line-strong);
  border-radius: 14px;
  background: var(--surface-muted);
}

input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent);
}

@media (max-width: 67.5rem) {
  .landing-layout,
  .admin-grid,
  .account-management-grid,
  .share-grid,
  .workspace-columns,
  .queue-page-layout {
    grid-template-columns: 1fr;
  }

  .landing-layout {
    align-items: start;
    min-height: auto;
  }

  .landing-copy {
    min-height: auto;
    padding-right: 0;
  }

  .auth-inline-grid {
    grid-template-columns: 1fr;
  }

  h1 {
    max-width: none;
  }
}

@media (max-width: 45rem) {
  .shell {
    padding: 1rem 1rem 3rem;
  }

  .landing-copy,
  .login-panel,
  .sidebar-panel,
  .workspace-panel,
  .workspace-header,
  .request-list-panel,
  .request-detail-panel,
  .empty-stage {
    padding: 18px;
  }

  .button-row,
  .action-bar,
  .panel-header,
  .signed-in-strip {
    align-items: stretch;
  }

  .button-row .button,
  .action-bar .button,
  .signed-in-strip .button {
    width: 100%;
  }

  .detail-row {
    grid-template-columns: 1fr;
    gap: 6px;
  }

  .field-editor-row {
    grid-template-columns: 1fr;
  }

  .account-card {
    grid-template-columns: 1fr;
    align-items: start;
  }

  .account-card-side {
    justify-content: flex-start;
  }

  .account-role-summary {
    grid-template-columns: 1fr;
  }
}
"#;
