<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useSettingsStore, type GlobalSettings } from "../stores/settings";
import ToggleSwitch from "../components/ToggleSwitch.vue";

const settingsStore = useSettingsStore();

const form = ref<GlobalSettings>({
  version: 1,
  terminology_server_url: null,
  check_updates_on_startup: true,
  analytics_enabled: false,
  analytics_consent_asked: false,
});
const saving = ref(false);
const saveResult = ref<string | null>(null);
const saveError = ref<string | null>(null);
const configDir = ref<string | null>(null);

onMounted(async () => {
  await settingsStore.loadSettings();
  form.value = { ...settingsStore.settings };
  // Load config directory path
  try {
    configDir.value = await invoke<string>("get_config_dir");
  } catch (e) {
    console.error("Failed to get config dir:", e);
  }
});

async function save() {
  saving.value = true;
  saveResult.value = null;
  saveError.value = null;
  try {
    // Convert empty string to null
    const toSave = {
      ...form.value,
      terminology_server_url: form.value.terminology_server_url?.trim() || null,
    };
    await settingsStore.saveSettings(toSave);
    saveResult.value = "Settings saved successfully.";
  } catch (e) {
    saveError.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function openConfigDir() {
  if (configDir.value) {
    await revealItemInDir(configDir.value);
  }
}
</script>

<template>
  <div class="settings-page">
    <div class="view-header">
      <h2>Global Settings</h2>
    </div>

    <div class="settings-form">
      <div class="settings-section">
        <h3>Terminology</h3>
        <div class="section-divider"></div>

        <div class="form-group">
          <label>Default terminology server URL</label>
          <input
            class="input"
            v-model="form.terminology_server_url"
            placeholder="https://tx.fhir.ch/r4"
          />
          <p class="form-help">
            Used for resolving external codes (SNOMED CT, LOINC, etc.) unless a server profile
            overrides it. Leave empty to disable code resolution globally.
          </p>
        </div>
      </div>

      <div class="settings-section">
        <h3>Updates</h3>
        <div class="section-divider"></div>

        <div class="form-group">
          <ToggleSwitch
            v-model="form.check_updates_on_startup"
            label="Check for updates on app startup"
          />
          <p class="form-help">
            When enabled, openEHR Explorer checks GitHub Releases on launch and shows a notification
            if a new version is available. No personal data is sent; only the current app version
            and platform are transmitted as part of the update request.
          </p>
        </div>
      </div>

      <div class="settings-section">
        <h3>Analytics</h3>
        <div class="section-divider"></div>

        <div class="form-group">
          <ToggleSwitch
            v-model="form.analytics_enabled"
            label="Help improve openEHR Explorer by sharing anonymous usage data"
          />
          <p class="form-help">
            When enabled, openEHR Explorer sends coarse-grained, anonymous feature-usage events
            to <a href="https://aptabase.com" target="_blank" rel="noopener">Aptabase</a>
            (a privacy-first, GDPR-compliant analytics service). We collect: app version, OS
            and architecture, which features are used (e.g. <em>aql_executed</em>,
            <em>composition_viewed</em>), and the CDR platform type
            (<em>ehrbase</em>/<em>better_platform</em>/<em>generic</em>).
          </p>
          <p class="form-help">
            We <strong>never</strong> collect server URLs, credentials, patient data, EHR IDs,
            composition content, query text, template content, or anything you type. No cookies,
            no fingerprinting, no IP logging. You can toggle this off at any time and event
            collection stops immediately. Default is <strong>off</strong>.
          </p>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn btn-primary" @click="save" :disabled="saving">
          {{ saving ? "Saving..." : "Save" }}
        </button>
      </div>

      <div v-if="saveResult" class="save-result success">{{ saveResult }}</div>
      <div v-if="saveError" class="save-result error">{{ saveError }}</div>
    </div>

    <!-- Settings Files Info -->
    <div v-if="configDir" class="config-info">
      <h3>Settings Files</h3>
      <p class="config-hint">Application data is stored in the following files:</p>
      <ul class="config-file-list">
        <li><code>profiles.json</code> &mdash; Server profiles</li>
        <li><code>saved_queries.json</code> &mdash; Saved AQL queries</li>
        <li><code>settings.json</code> &mdash; Global settings</li>
      </ul>
      <p class="config-path">{{ configDir }}</p>
      <button class="btn btn-sm" @click="openConfigDir">Open Directory</button>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  padding: 24px;
  max-width: 700px;
}

.view-header {
  margin-bottom: 24px;
}
.view-header h2 {
  font-size: 20px;
  font-weight: 600;
}

.settings-form {
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.settings-section h3 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 8px;
}

.section-divider {
  height: 1px;
  background: var(--color-border);
  margin-bottom: 16px;
}

.form-group {
  margin-bottom: 16px;
}
.form-group label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 4px;
}
.form-group .input {
  width: 100%;
}
.form-group .checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  color: var(--color-text);
  font-weight: 500;
}
.form-group .checkbox-label input[type="checkbox"] {
  cursor: pointer;
}
.form-help {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-muted);
  line-height: 1.4;
}

.form-actions {
  margin-top: 20px;
}

.save-result {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: var(--radius);
  font-size: 13px;
}
.save-result.success {
  background: rgba(107, 255, 142, 0.1);
  color: var(--color-success);
  border: 1px solid var(--color-success);
}
.save-result.error {
  background: rgba(255, 107, 107, 0.1);
  color: var(--color-error);
  border: 1px solid var(--color-error);
}

.config-info {
  margin-top: 24px;
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}
.config-info h3 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 8px;
}
.config-hint {
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-bottom: 8px;
  line-height: 1.4;
}
.config-file-list {
  list-style: none;
  padding: 0;
  margin: 0 0 10px 0;
}
.config-file-list li {
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.6;
}
.config-file-list code {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-text);
}
.config-path {
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--color-text-muted);
  word-break: break-all;
  margin-bottom: 10px;
  line-height: 1.4;
}
</style>
