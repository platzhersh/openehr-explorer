<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useServerStore, type ServerProfile } from "../stores/server";
import { useSettingsStore } from "../stores/settings";

const serverStore = useServerStore();
const settingsStore = useSettingsStore();

const globalTerminologyUrl = computed(() => settingsStore.settings.terminology_server_url || "");

const editing = ref(false);
const testResult = ref<string | null>(null);
const testError = ref<string | null>(null);

const form = ref<ServerProfile>({
  id: "",
  name: "",
  base_url: "",
  server_type: "ehrbase",
  auth_method: { type: "none" },
});

onMounted(async () => {
  await serverStore.loadProfiles();
  // Fetch version info for all profiles
  for (const profile of serverStore.profiles) {
    const version = await serverStore.fetchServerVersion(profile);
    console.log(`Version for ${profile.name}:`, version);
  }
});

function newProfile() {
  form.value = {
    id: crypto.randomUUID(),
    name: "",
    base_url: "http://localhost:8080/ehrbase",
    server_type: "ehrbase",
    auth_method: { type: "basic", username: "ehrbase-user", password: "SuperSecretPassword" },
    terminology_url: null,
  };
  editing.value = true;
  testResult.value = null;
  testError.value = null;
}

function editProfile(profile: ServerProfile) {
  form.value = {
    ...profile,
    auth_method: { ...profile.auth_method } as any,
    admin_auth_method: profile.admin_auth_method ? ({ ...profile.admin_auth_method } as any) : null,
    terminology_url: profile.terminology_url || null,
  };
  editing.value = true;
  testResult.value = null;
  testError.value = null;
}

async function save() {
  await serverStore.saveProfile(form.value);
  editing.value = false;
}

async function remove(id: string) {
  await serverStore.deleteProfile(id);
}

async function testConnection() {
  testResult.value = null;
  testError.value = null;
  try {
    testResult.value = await serverStore.testConnection(form.value);
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
</script>

<template>
  <div class="server-manager">
    <div class="view-header">
      <h2>Server Profiles</h2>
      <button class="btn btn-primary" @click="newProfile">+ Add Server</button>
    </div>

    <div class="content-area">
      <!-- Profile list -->
      <div class="profile-list">
        <div
          v-for="profile in serverStore.profiles"
          :key="profile.id"
          class="profile-card"
          :class="{ active: profile.id === serverStore.activeServerId }"
        >
          <div class="profile-info">
            <div class="profile-name">{{ profile.name }}</div>
            <div class="profile-url">{{ profile.base_url }}</div>
            <div class="profile-meta">
              <span class="badge">{{ profile.server_type }}</span>
              <span class="badge">{{ profile.auth_method.type }}</span>
              <span
                v-if="serverStore.versionInfo[profile.id]?.ehrbase_version"
                class="badge version-badge"
                :title="`EHRbase ${serverStore.versionInfo[profile.id]?.ehrbase_version}`"
              >
                v{{ serverStore.versionInfo[profile.id]?.ehrbase_version }}
              </span>
            </div>
          </div>
          <div class="profile-actions">
            <button class="btn btn-sm" @click="serverStore.setActiveServer(profile.id)">
              {{ profile.id === serverStore.activeServerId ? "Active" : "Use" }}
            </button>
            <button class="btn btn-sm" @click="editProfile(profile)">Edit</button>
            <button class="btn btn-sm btn-danger" @click="remove(profile.id)">Delete</button>
          </div>
        </div>

        <div v-if="serverStore.profiles.length === 0" class="empty-state">
          <h3>No servers configured</h3>
          <p>Add a server profile to connect to an openEHR CDR.</p>
        </div>
      </div>

      <!-- Edit form -->
      <div v-if="editing" class="profile-form">
        <h3>{{ form.id ? "Edit" : "Add" }} Server Profile</h3>

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
          />
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
            <input class="input" type="password" v-model="(form.auth_method as any).password" />
          </div>
        </template>

        <template v-if="form.auth_method.type === 'bearer'">
          <div class="form-group">
            <label>Token</label>
            <input class="input" v-model="(form.auth_method as any).token" />
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
              />
            </div>
          </template>

          <template v-if="form.admin_auth_method?.type === 'bearer'">
            <div class="form-group">
              <label>Admin Token</label>
              <input class="input" v-model="(form.admin_auth_method as any).token" />
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

        <div class="form-actions">
          <button class="btn" @click="testConnection">Test Connection</button>
          <button class="btn btn-primary" @click="save">Save</button>
          <button class="btn" @click="editing = false">Cancel</button>
        </div>

        <div v-if="testResult" class="test-result success">{{ testResult }}</div>
        <div v-if="testError" class="test-result error">{{ testError }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.server-manager {
  padding: 24px;
  max-width: 900px;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.view-header h2 {
  font-size: 20px;
  font-weight: 600;
}

.content-area {
  display: flex;
  gap: 24px;
}

.profile-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.profile-card {
  padding: 16px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.profile-card.active {
  border-color: var(--color-primary-dim);
}

.profile-name {
  font-weight: 600;
  margin-bottom: 4px;
}
.profile-url {
  font-size: 12px;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  margin-bottom: 6px;
}
.profile-meta {
  display: flex;
  gap: 6px;
}
.version-badge {
  background: var(--color-primary-dim);
  color: var(--color-primary);
  font-weight: 600;
}
.profile-actions {
  display: flex;
  gap: 6px;
}

.profile-form {
  flex: 1;
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}
.profile-form h3 {
  margin-bottom: 16px;
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

.form-divider {
  border: none;
  border-top: 1px solid var(--color-border);
  margin: 16px 0;
}

.form-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
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
</style>
