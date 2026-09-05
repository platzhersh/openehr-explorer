<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useServerStore, type ServerProfile } from "../stores/server";
import { useAnalytics } from "../composables/useAnalytics";
import { useTourStore } from "../stores/tour";
import ServerFormDialog from "../components/ServerFormDialog.vue";
import CompassIcon from "../components/CompassIcon.vue";
import PlusIcon from "../components/PlusIcon.vue";
import EditButton from "../components/EditButton.vue";
import DeleteButton from "../components/DeleteButton.vue";

const serverStore = useServerStore();
const analytics = useAnalytics();
const tourStore = useTourStore();

const cardTestLoading = ref<Record<string, boolean>>({});
const cardTestResult = ref<Record<string, { success: boolean; message: string }>>({});

const dialogOpen = ref(false);
const dialogProfile = ref<ServerProfile | null>(null);

onMounted(async () => {
  await serverStore.loadProfiles();
  for (const profile of serverStore.profiles) {
    const version = await serverStore.fetchServerVersion(profile.id);
    console.log(`Version for ${profile.name}:`, version);
  }
});

function newProfile() {
  dialogProfile.value = null;
  dialogOpen.value = true;
}

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: "servers" });
  tourStore.start("servers");
}

function editProfile(profile: ServerProfile) {
  dialogProfile.value = profile;
  dialogOpen.value = true;
}

function closeDialog() {
  dialogOpen.value = false;
  dialogProfile.value = null;
}

async function remove(id: string) {
  await serverStore.deleteProfile(id);
  void analytics.track("server_profile_deleted");
}

async function toggleDefault(profile: ServerProfile) {
  await serverStore.setDefaultProfile(profile.id);
}

async function testProfileConnection(profile: ServerProfile) {
  cardTestLoading.value[profile.id] = true;
  delete cardTestResult.value[profile.id];
  try {
    const result = await serverStore.testConnection(profile.id);
    cardTestResult.value[profile.id] = { success: true, message: result };
  } catch (e) {
    cardTestResult.value[profile.id] = { success: false, message: String(e) };
  } finally {
    cardTestLoading.value[profile.id] = false;
  }
}

function isInsecureHttpUrl(url: string): boolean {
  try {
    const parsedUrl = new URL(url);
    if (parsedUrl.protocol !== "http:") {
      return false;
    }

    const isLocalhost =
      parsedUrl.hostname === "localhost" ||
      parsedUrl.hostname === "127.0.0.1" ||
      parsedUrl.hostname === "::1" ||
      parsedUrl.hostname.startsWith("192.168.") ||
      parsedUrl.hostname.startsWith("10.") ||
      parsedUrl.hostname.match(/^172\.(1[6-9]|2[0-9]|3[0-1])\./);

    return !isLocalhost;
  } catch {
    return false;
  }
}

function credentialBackendLabel(backend: string): string {
  switch (backend) {
    case "os_keychain":
      return "OS Keychain";
    case "encrypted_file":
      return "Encrypted File";
    default:
      return backend;
  }
}
</script>

<template>
  <div class="server-manager">
    <div class="view-header">
      <h2>Server Profiles</h2>
      <div class="header-actions">
        <button
          type="button"
          class="tour-trigger-btn"
          title="Take a tour of the Server Manager"
          @click="replayTour"
        >
          <CompassIcon />
        </button>
        <button
          type="button"
          class="btn btn-sm btn-primary"
          data-tour="server-add"
          @click="newProfile"
        >
          <PlusIcon />
          Add Server
        </button>
      </div>
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
                v-if="serverStore.versionInfo[profile.id]?.server_version"
                class="badge version-badge"
                :title="serverStore.versionInfo[profile.id]?.server_version ?? ''"
              >
                v{{ serverStore.versionInfo[profile.id]?.server_version }}
              </span>
              <span
                v-if="isInsecureHttpUrl(profile.base_url)"
                class="badge warning-badge"
                title="Using HTTP for a remote server (credentials sent unencrypted)"
              >
                ⚠️ HTTP
              </span>
              <span
                class="badge secure-badge"
                :title="`Credentials stored via ${credentialBackendLabel(profile.credential_backend)}`"
              >
                🔒 {{ credentialBackendLabel(profile.credential_backend) }}
              </span>
            </div>
            <div
              v-if="cardTestResult[profile.id]"
              class="card-test-result"
              :class="cardTestResult[profile.id].success ? 'success' : 'error'"
            >
              {{ cardTestResult[profile.id].message }}
            </div>
          </div>
          <div class="profile-actions">
            <button
              class="btn btn-sm"
              @click="testProfileConnection(profile)"
              :disabled="cardTestLoading[profile.id]"
            >
              {{ cardTestLoading[profile.id] ? "Testing..." : "Test" }}
            </button>
            <button class="btn btn-sm" @click="serverStore.setActiveServer(profile.id)">
              {{ profile.id === serverStore.activeServerId ? "Active" : "Use" }}
            </button>
            <button
              type="button"
              class="btn btn-sm"
              :class="{ 'btn-active-toggle': profile.is_default }"
              title="Preselect this server on app start"
              @click="toggleDefault(profile)"
            >
              {{ profile.is_default ? "★ Default" : "Set as Default" }}
            </button>
            <EditButton
              title="Edit server profile"
              variant="bordered"
              size="md"
              @click="editProfile(profile)"
            />
            <DeleteButton
              title="Delete server profile"
              variant="bordered"
              size="md"
              @click="remove(profile.id)"
            />
          </div>
        </div>

        <div v-if="serverStore.profiles.length === 0" class="empty-state">
          <h3>No servers configured</h3>
          <p>Add a server profile to connect to an openEHR CDR.</p>
        </div>
      </div>
    </div>

    <ServerFormDialog :open="dialogOpen" :profile="dialogProfile" @close="closeDialog" />
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
.view-header .header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
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
  flex-wrap: wrap;
}
.version-badge {
  background: var(--color-primary-dim);
  color: var(--color-primary);
  font-weight: 600;
}
.btn-active-toggle {
  border-color: #eab308;
  color: #eab308;
}
.btn-active-toggle:hover {
  background: #eab308;
  color: var(--color-bg);
}
.warning-badge {
  background: rgba(255, 193, 7, 0.15);
  color: #f59e0b;
  font-weight: 600;
  border: 1px solid #fbbf24;
}
.secure-badge {
  background: rgba(34, 197, 94, 0.1);
  color: #22c55e;
  font-weight: 600;
  border: 1px solid rgba(34, 197, 94, 0.3);
}
.card-test-result {
  margin-top: 6px;
  font-size: 12px;
  line-height: 1.4;
}
.card-test-result.success {
  color: var(--color-success);
}
.card-test-result.error {
  color: var(--color-error);
}
.profile-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.empty-state {
  padding: 40px 20px;
  text-align: center;
  color: var(--color-text-secondary);
}
.empty-state h3 {
  margin-bottom: 8px;
}
</style>
