<script setup lang="ts">
// First-party downloads / stars pills. These used to be shields.io
// images, which break whenever shields.io's GitHub token pool is
// exhausted ("Unable to select next GitHub token from pool") or its
// GitHub integration errors out ("invalid"). Both numbers are already
// reachable without a third party: stars from the public GitHub API,
// downloads from the daily snapshot history the sparkline also reads.
// A pill that can't load is simply hidden rather than showing an error.
import { onMounted, ref } from "vue";

const REPO_URL = "https://github.com/platzhersh/openehr-explorer";
const STARS_URL = "https://api.github.com/repos/platzhersh/openehr-explorer";
const HISTORY_URL = "https://raw.githubusercontent.com/platzhersh/openehr-explorer/data/docs/downloads-history.json";

const downloads = ref<number | null>(null);
const stars = ref<number | null>(null);

function format(n: number) {
  return n.toLocaleString("en-US");
}

onMounted(() => {
  fetch(HISTORY_URL)
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      if (!Array.isArray(data)) return;
      const total = data[data.length - 1]?.total;
      if (Number.isFinite(total)) downloads.value = total;
    })
    .catch((err) => console.debug("repo badges: downloads unavailable", err));

  fetch(STARS_URL)
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      if (Number.isFinite(data?.stargazers_count)) stars.value = data.stargazers_count;
    })
    .catch((err) => console.debug("repo badges: stars unavailable", err));
});
</script>

<template>
  <p class="repo-badges">
    <a v-if="downloads !== null" class="repo-badge" :href="`${REPO_URL}/releases`">
      <span class="repo-badge-label">downloads</span>
      <span class="repo-badge-value">{{ format(downloads) }}</span>
    </a>
    <a v-if="stars !== null" class="repo-badge" :href="REPO_URL">
      <span class="repo-badge-label">github stars</span>
      <span class="repo-badge-value">{{ format(stars) }}</span>
    </a>
  </p>
</template>

<style>
.repo-badges {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  justify-content: center;
  min-height: 20px;
}
.repo-badge {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
  font-size: 0.75rem;
  line-height: 1;
  text-decoration: none;
}
.repo-badge-label,
.repo-badge-value {
  padding: 4px 8px;
}
.repo-badge-label {
  background: var(--surface);
  color: var(--text-secondary);
}
.repo-badge-value {
  background: var(--primary-bg);
  color: var(--primary);
  font-weight: 600;
}
.repo-badge:hover .repo-badge-value {
  background: var(--primary-border);
}
</style>
