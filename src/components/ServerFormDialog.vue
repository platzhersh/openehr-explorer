<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useServerStore, type ServerProfile, type ServerProfileInput } from "../stores/server";
import { useSettingsStore } from "../stores/settings";
import { useAnalytics } from "../composables/useAnalytics";

const props = defineProps<{
  open: boolean;
  profile: ServerProfile | null;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const serverStore = useServerStore();
const settingsStore = useSettingsStore();
const analytics = useAnalytics();

const globalTerminologyUrl = computed(() => settingsStore.settings.terminology_server_url || "");

const editingExistingId = ref<string | null>(null);
const testResult = ref<string | null>(null);
const testError = ref<string | null>(null);
const urlValidationError = ref<string | null>(null);
const urlValidationWarning = ref<string | null>(null);
const saving = ref(false);

const form = ref<ServerProfileInput>({
  id: "",
  name: "",
  base_url: "",
  server_type: "ehrbase",
  auth_method: { type: "none" },
});

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      if (props.profile) {
        initEditForm(props.profile);
      } else {
        initNewForm();
      }
    }
  },
);

function initNewForm() {
  form.value = {
    id: crypto.randomUUID(),
    name: "",
    base_url: "http://localhost:8080/ehrbase",
    server_type: "ehrbase",
    auth_method: { type: "basic", username: "", password: "" },
    terminology_url: null,
  };
  editingExistingId.value = null;
  testResult.value = null;
  testError.value = null;
  urlValidationError.value = null;
  urlValidationWarning.value = null;
}

function initEditForm(profile: ServerProfile) {
  // Convert public profile to input form. Secret fields are empty
  // (user must re-enter them to change; otherwise backend keeps existing secrets).
  form.value = {
    id: profile.id,
    name: profile.name,
    base_url: profile.base_url,
    server_type: profile.server_type,
    auth_method: publicAuthToInput(profile.auth_method),
    admin_auth_method: profile.admin_auth_method
      ? publicAuthToInput(profile.admin_auth_method)
      : null,
    terminology_url: profile.terminology_url || null,
  };
  editingExistingId.value = profile.id;
  testResult.value = null;
  testError.value = null;
  urlValidationError.value = null;
  urlValidationWarning.value = null;
  validateBaseUrl(profile.base_url);
}

/** Convert a public auth (no secrets) to an input auth (with empty secret fields). */
function publicAuthToInput(
  auth:
    | { type: "none" }
    | { type: "basic"; username: string; has_password: boolean }
    | { type: "bearer"; has_token: boolean },
): ServerProfileInput["auth_method"] {
  switch (auth.type) {
    case "none":
      return { type: "none" };
    case "basic":
      return { type: "basic", username: auth.username, password: "" };
    case "bearer":
      return { type: "bearer", token: "" };
  }
}

function validateBaseUrl(url: string): boolean {
  urlValidationError.value = null;
  urlValidationWarning.value = null;

  if (!url || url.trim() === "") {
    urlValidationError.value = "Base URL is required";
    return false;
  }

  let parsedUrl: URL;
  try {
    parsedUrl = new URL(url);
  } catch {
    urlValidationError.value = "Invalid URL format";
    return false;
  }

  const dangerousSchemes = ["javascript", "data", "file", "vbscript", "about"];
  if (dangerousSchemes.includes(parsedUrl.protocol.replace(":", ""))) {
    urlValidationError.value = `Dangerous URL scheme '${parsedUrl.protocol}' is not allowed`;
    return false;
  }

  if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
    urlValidationError.value = "URL must use http:// or https:// protocol";
    return false;
  }

  if (parsedUrl.protocol === "http:") {
    const isLocalhost =
      parsedUrl.hostname === "localhost" ||
      parsedUrl.hostname === "127.0.0.1" ||
      parsedUrl.hostname === "::1" ||
      parsedUrl.hostname.startsWith("192.168.") ||
      parsedUrl.hostname.startsWith("10.") ||
      parsedUrl.hostname.match(/^172\.(1[6-9]|2[0-9]|3[0-1])\./);

    if (!isLocalhost) {
      urlValidationWarning.value =
        "Using HTTP for a remote server will send credentials unencrypted. Consider using HTTPS.";
    }
  }

  return true;
}

async function save() {
  if (!validateBaseUrl(form.value.base_url)) {
    return;
  }
  // Snapshot before save — editingExistingId is null when creating a brand
  // new profile, so we use it to distinguish created-vs-updated. Updates are
  // deliberately NOT tracked to keep event volume down; only the initial
  // create is recorded.
  const wasCreating = editingExistingId.value === null;
  const serverType = form.value.server_type;
  testResult.value = null;
  testError.value = null;
  saving.value = true;
  try {
    await serverStore.saveProfile(form.value);
    if (wasCreating) {
      // Platform-distribution signal — server_type is a known enum so it's
      // safe to include. Never URL, name, or auth material.
      void analytics.track("server_profile_created", { server_type: serverType });
    } else {
      void analytics.track("server_profile_updated", { server_type: serverType });
    }
    emit("saved");
    handleClose();
  } catch (e) {
    testError.value = `Save failed: ${String(e)}`;
  } finally {
    saving.value = false;
  }
}

async function testConnection() {
  testResult.value = null;
  testError.value = null;
  try {
    // For unsaved profiles (or profiles being edited), use unsaved connection test
    testResult.value = await serverStore.testUnsavedConnection(form.value);
  } catch (e) {
    testError.value = String(e);
  }
}

function setAuthType(type: string) {
  if (type === "none") {
    form.value.auth_method = { type: "none" };
  } else if (type === "basic") {
    form.value.auth_method = { type: "basic", username: "", password: "" };
  } else if (type === "bearer") {
    form.value.auth_method = { type: "bearer", token: "" };
  }
}

function setAdminAuthType(type: string) {
  if (type === "none_admin") {
    form.value.admin_auth_method = null;
  } else if (type === "basic") {
    form.value.admin_auth_method = { type: "basic", username: "", password: "" };
  } else if (type === "bearer") {
    form.value.admin_auth_method = { type: "bearer", token: "" };
  }
}

/** Check if we are editing a profile that already has a saved secret for a field. */
function existingProfileHasPassword(): boolean {
  if (!editingExistingId.value) return false;
  const existing = serverStore.profiles.find((p) => p.id === editingExistingId.value);
  if (!existing) return false;
  return existing.auth_method.type === "basic" && existing.auth_method.has_password;
}

function existingProfileHasToken(): boolean {
  if (!editingExistingId.value) return false;
  const existing = serverStore.profiles.find((p) => p.id === editingExistingId.value);
  if (!existing) return false;
  return existing.auth_method.type === "bearer" && existing.auth_method.has_token;
}

function existingProfileHasAdminPassword(): boolean {
  if (!editingExistingId.value) return false;
  const existing = serverStore.profiles.find((p) => p.id === editingExistingId.value);
  if (!existing || !existing.admin_auth_method) return false;
  return existing.admin_auth_method.type === "basic" && existing.admin_auth_method.has_password;
}

function existingProfileHasAdminToken(): boolean {
  if (!editingExistingId.value) return false;
  const existing = serverStore.profiles.find((p) => p.id === editingExistingId.value);
  if (!existing || !existing.admin_auth_method) return false;
  return existing.admin_auth_method.type === "bearer" && existing.admin_auth_method.has_token;
}

function handleClose() {
  testResult.value = null;
  testError.value = null;
  emit("close");
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>{{ editingExistingId ? "Edit" : "Add" }} Server Profile</h2>
        <button class="close-btn" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <form @submit.prevent="save">
          <div class="form-group">
            <label>Name</label>
            <input class="input" v-model="form.name" placeholder="My EHRBase" />
          </div>

          <div class="form-group">
            <label>Base URL</label>
            <input
              class="input"
              v-model="form.base_url"
              placeholder="http://localhost:8080/ehrbase"
              @input="validateBaseUrl(form.base_url)"
              :class="{ 'input-error': urlValidationError, 'input-warning': urlValidationWarning }"
            />
            <div v-if="urlValidationError" class="validation-message error">
              {{ urlValidationError }}
            </div>
            <div
              v-if="urlValidationWarning && !urlValidationError"
              class="validation-message warning"
            >
              ⚠️ {{ urlValidationWarning }}
            </div>
          </div>

          <div class="form-group">
            <label>Server Type</label>
            <select class="input" v-model="form.server_type">
              <option value="ehrbase">EHRBase</option>
              <option value="better_platform">Better Platform</option>
              <option value="generic">Generic openEHR REST</option>
            </select>
          </div>

          <div class="form-group">
            <label>Authentication</label>
            <select
              class="input"
              :value="form.auth_method.type"
              @change="setAuthType(($event.target as HTMLSelectElement).value)"
            >
              <option value="none">None</option>
              <option value="basic">Basic Auth</option>
              <option value="bearer">Bearer Token</option>
            </select>
          </div>

          <template v-if="form.auth_method.type === 'basic'">
            <div class="form-group">
              <label>Username</label>
              <input class="input" v-model="(form.auth_method as any).username" />
            </div>
            <div class="form-group">
              <label>Password</label>
              <input
                class="input"
                type="password"
                v-model="(form.auth_method as any).password"
                :placeholder="
                  existingProfileHasPassword()
                    ? 'Password saved securely (leave empty to keep)'
                    : ''
                "
              />
              <p v-if="existingProfileHasPassword()" class="form-help secure-hint">
                🔒 Password is stored securely. Leave empty to keep the existing password.
              </p>
            </div>
          </template>

          <template v-if="form.auth_method.type === 'bearer'">
            <div class="form-group">
              <label>Token</label>
              <input
                class="input"
                v-model="(form.auth_method as any).token"
                :placeholder="
                  existingProfileHasToken() ? 'Token saved securely (leave empty to keep)' : ''
                "
              />
              <p v-if="existingProfileHasToken()" class="form-help secure-hint">
                🔒 Token is stored securely. Leave empty to keep the existing token.
              </p>
            </div>
          </template>

          <template v-if="form.server_type === 'ehrbase'">
            <hr class="form-divider" />
            <div class="form-group">
              <label>Admin Credentials (for EHR deletion)</label>
              <select
                class="input"
                :value="form.admin_auth_method ? form.admin_auth_method.type : 'none_admin'"
                @change="setAdminAuthType(($event.target as HTMLSelectElement).value)"
              >
                <option value="none_admin">Same as above</option>
                <option value="basic">Basic Auth</option>
                <option value="bearer">Bearer Token</option>
              </select>
            </div>

            <template v-if="form.admin_auth_method?.type === 'basic'">
              <div class="form-group">
                <label>Admin Username</label>
                <input class="input" v-model="(form.admin_auth_method as any).username" />
              </div>
              <div class="form-group">
                <label>Admin Password</label>
                <input
                  class="input"
                  type="password"
                  v-model="(form.admin_auth_method as any).password"
                  :placeholder="
                    existingProfileHasAdminPassword()
                      ? 'Password saved securely (leave empty to keep)'
                      : ''
                  "
                />
                <p v-if="existingProfileHasAdminPassword()" class="form-help secure-hint">
                  🔒 Password is stored securely. Leave empty to keep the existing password.
                </p>
              </div>
            </template>

            <template v-if="form.admin_auth_method?.type === 'bearer'">
              <div class="form-group">
                <label>Admin Token</label>
                <input
                  class="input"
                  v-model="(form.admin_auth_method as any).token"
                  :placeholder="
                    existingProfileHasAdminToken()
                      ? 'Token saved securely (leave empty to keep)'
                      : ''
                  "
                />
                <p v-if="existingProfileHasAdminToken()" class="form-help secure-hint">
                  🔒 Token is stored securely. Leave empty to keep the existing token.
                </p>
              </div>
            </template>
          </template>

          <hr class="form-divider" />
          <div class="form-group">
            <label>Terminology Server URL (optional)</label>
            <input
              class="input"
              v-model="form.terminology_url"
              :placeholder="
                globalTerminologyUrl
                  ? `Using global default: ${globalTerminologyUrl}`
                  : 'No global default configured'
              "
            />
            <p class="form-help">
              Overrides the global default for this profile. Leave empty to use the global setting.
            </p>
          </div>

          <div v-if="testResult" class="test-result success">{{ testResult }}</div>
          <div v-if="testError" class="test-result error">{{ testError }}</div>

          <div class="dialog-actions">
            <button type="button" class="btn" @click="testConnection">Test Connection</button>
            <button type="button" class="btn btn-secondary" @click="handleClose">Cancel</button>
            <button type="submit" class="btn btn-primary" :disabled="saving">
              {{ saving ? "Saving..." : "Save" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: var(--color-bg);
  border-radius: var(--radius);
  width: 90%;
  max-width: 600px;
  max-height: 90vh;
  overflow: auto;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-border);
}

.dialog-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  font-size: 28px;
  color: var(--color-text-muted);
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}
.close-btn:hover {
  background: var(--color-surface);
}

.dialog-body {
  padding: 24px;
}

.form-group {
  margin-bottom: 12px;
}
.form-group label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 4px;
}
.form-group .input {
  width: 100%;
}

.form-help {
  margin-top: 4px;
  font-size: 12px;
  color: var(--color-text-muted);
  line-height: 1.4;
}

.secure-hint {
  color: #22c55e;
}

.form-divider {
  border: none;
  border-top: 1px solid var(--color-border);
  margin: 16px 0;
}

.test-result {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: var(--radius);
  font-size: 13px;
}
.test-result.success {
  background: rgba(107, 255, 142, 0.1);
  color: var(--color-success);
  border: 1px solid var(--color-success);
}
.test-result.error {
  background: rgba(255, 107, 107, 0.1);
  color: var(--color-error);
  border: 1px solid var(--color-error);
}

.validation-message {
  margin-top: 6px;
  padding: 6px 10px;
  border-radius: var(--radius);
  font-size: 12px;
  line-height: 1.4;
}
.validation-message.error {
  background: rgba(255, 107, 107, 0.1);
  color: var(--color-error);
  border: 1px solid var(--color-error);
}
.validation-message.warning {
  background: rgba(255, 193, 7, 0.1);
  color: #f59e0b;
  border: 1px solid #fbbf24;
}

.input-error {
  border-color: var(--color-error) !important;
}
.input-warning {
  border-color: #fbbf24 !important;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}
</style>
