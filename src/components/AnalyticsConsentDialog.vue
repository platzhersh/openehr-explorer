<script setup lang="ts">
/**
 * AnalyticsConsentDialog — first-run modal shown exactly once, when the
 * persisted settings file has no prior analytics decision
 * (`analytics_consent_asked === false`).
 *
 * The parent (`App.vue`) decides when to mount this; after the user picks a
 * choice the component emits either `accept` or `decline` and the parent
 * writes the resulting settings back to disk (setting both
 * `analytics_enabled` and `analytics_consent_asked` in one save so we never
 * re-prompt).
 *
 * Styling intentionally mirrors `EhrCreateDialog.vue` for visual
 * consistency. No close button — the user MUST pick a side so we don't get
 * stuck in an ambiguous "asked but not answered" state.
 */

defineEmits<{
  (e: "accept"): void;
  (e: "decline"): void;
}>();
</script>

<template>
  <div class="dialog-overlay">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="consent-title">
      <div class="dialog-header">
        <h2 id="consent-title">Help shape openEHR Explorer</h2>
      </div>

      <div class="dialog-body">
        <p class="lead">
          Would you like to share <strong>anonymous</strong> usage data with the openEHR Explorer
          project? It helps us understand which features matter and where to focus development.
        </p>

        <div class="info-block positive">
          <div class="info-heading">
            <span class="info-icon" aria-hidden="true">✓</span>
            <span>What we collect</span>
          </div>
          <ul class="info-list">
            <li>App version, OS, and architecture</li>
            <li>
              Which features you use (e.g. <em>aql_executed</em>, <em>composition_viewed</em>)
            </li>
            <li>
              The CDR platform type (<em>ehrbase</em> / <em>better_platform</em> / <em>generic</em>)
            </li>
          </ul>
        </div>

        <div class="info-block negative">
          <div class="info-heading">
            <span class="info-icon" aria-hidden="true">✕</span>
            <span>What we <strong>never</strong> collect</span>
          </div>
          <ul class="info-list">
            <li>Server URLs, credentials, or IP addresses</li>
            <li>Patient data, EHR IDs, or composition content</li>
            <li>Query text, template content, or anything you type</li>
          </ul>
        </div>

        <p class="footnote">
          <strong>Even if you say no</strong>, the app sends a single <em>session_started</em> ping
          once per launch so we can count total sessions and see how many people opt in. It contains
          only one field &mdash; your consent choice (yes/no) &mdash; and nothing else: no version,
          no OS, no feature usage.
        </p>
        <p class="footnote">
          Data is sent to
          <a href="https://aptabase.com" target="_blank" rel="noopener">Aptabase</a>, a
          privacy-first, GDPR-compliant analytics service. No cookies, no fingerprinting.
        </p>
        <p class="footnote hint">
          You can change this at any time under <strong>Settings → Analytics</strong>.
        </p>
      </div>

      <div class="dialog-footer">
        <button type="button" class="btn" @click="$emit('decline')">No thanks</button>
        <button type="button" class="btn btn-primary" @click="$emit('accept')">
          Yes, share anonymous data
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000; /* above UpdateNotification and RequestInspector */
}

.dialog {
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  width: 90%;
  max-width: 520px;
  max-height: 90vh;
  overflow: auto;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
}

.dialog-header {
  padding: 20px 24px 0 24px;
}
.dialog-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
  color: var(--color-primary);
}

.dialog-body {
  padding: 16px 24px 8px 24px;
}

.lead {
  font-size: 14px;
  color: var(--color-text);
  line-height: 1.5;
  margin: 0 0 16px 0;
}

.info-block {
  padding: 12px 14px;
  border-radius: var(--radius);
  margin-bottom: 12px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
}
.info-block.positive {
  border-color: var(--color-primary-dim);
}
.info-block.negative {
  border-color: var(--color-error);
  background: rgba(255, 107, 107, 0.06);
}

.info-heading {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 6px;
}

.info-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}
.positive .info-icon {
  background: var(--color-primary-dim);
  color: #fff;
}
.negative .info-icon {
  background: var(--color-error);
  color: #fff;
}

.info-list {
  margin: 0;
  padding-left: 28px;
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.6;
}
.info-list li em {
  font-family: var(--font-mono);
  font-style: normal;
  color: var(--color-text);
  font-size: 11px;
}

.footnote {
  font-size: 12px;
  color: var(--color-text-muted);
  line-height: 1.5;
  margin: 8px 0 0 0;
}
.footnote.hint {
  color: var(--color-text-secondary);
}
.footnote a {
  color: var(--color-primary);
  text-decoration: none;
}
.footnote a:hover {
  text-decoration: underline;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 20px 24px;
  border-top: 1px solid var(--color-border);
  margin-top: 8px;
}
</style>
