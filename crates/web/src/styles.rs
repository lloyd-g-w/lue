pub const APP_CSS: &str = r#"
:root {
  --bg: #f3f0ea;
  --surface: #fbfaf6;
  --surface-raised: #ffffff;
  --surface-muted: #ece6dc;
  --ink: #141312;
  --muted: #6a645c;
  --line: #d9d1c4;
  --line-strong: #bfb3a2;
  --accent: #18654e;
  --accent-soft: #dfebe6;
  --success: #2b7447;
  --danger: #b0404c;
  --warning: #8e691d;
  --left: #756d64;
  --shadow: 0 10px 30px rgba(20, 19, 18, 0.05);
  --radius: 20px;
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
  max-width: 1440px;
  margin: 0 auto;
  padding: 24px 24px 64px;
}

.landing-layout,
.admin-grid,
.workspace-columns,
.queue-page-layout {
  display: grid;
  gap: 24px;
}

.landing-layout {
  grid-template-columns: 1.2fr minmax(360px, 440px);
  align-items: start;
  min-height: calc(100vh - 120px);
}

.admin-shell {
  display: grid;
  gap: 20px;
}

.admin-grid {
  grid-template-columns: 380px minmax(0, 1fr);
  align-items: start;
}

.workspace-columns {
  grid-template-columns: minmax(360px, 440px) minmax(0, 1fr);
  gap: 20px;
}

.queue-page-layout {
  grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
  align-items: start;
}

.landing-copy,
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

.login-panel,
.request-detail-panel,
.queue-form-panel {
  background: var(--surface-raised);
}

.landing-copy {
  min-height: 540px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 28px;
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
  gap: 18px;
}

.sidebar-block,
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
  letter-spacing: 0.1em;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--muted);
}

h1,
h2,
h3,
.page-title {
  margin: 0;
  line-height: 1.08;
  letter-spacing: -0.04em;
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
  letter-spacing: -0.02em;
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
  padding: 12px 14px;
  font-weight: 700;
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
  grid-template-columns: 170px minmax(0, 1fr);
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

.request-list-shell {
  max-height: 680px;
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
}

.floating-feedback {
  margin-top: 4px;
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

.data-table {
  width: 100%;
  border-collapse: collapse;
  min-width: 760px;
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
  letter-spacing: 0.08em;
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

.table-action {
  padding: 10px 12px;
}

.table-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent);
}

@media (max-width: 1080px) {
  .landing-layout,
  .admin-grid,
  .workspace-columns,
  .queue-page-layout {
    grid-template-columns: 1fr;
  }

  h1 {
    max-width: none;
  }
}

@media (max-width: 720px) {
  .shell {
    padding: 16px 16px 48px;
  }

  .landing-copy,
  .login-panel,
  .sidebar-panel,
  .workspace-panel,
  .workspace-header,
  .request-list-panel,
  .request-detail-panel,
  .queue-hero-panel,
  .queue-form-panel,
  .empty-stage {
    padding: 18px;
  }

  .detail-row {
    grid-template-columns: 1fr;
    gap: 6px;
  }
}
"#;
