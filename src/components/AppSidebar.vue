<script setup lang="ts">
import { useRoute } from "vue-router";

const route = useRoute();

const navItems = [
  { path: "/ehrs", label: "EHR Browser", icon: "H" },
  { path: "/templates", label: "Templates", icon: "T" },
  { path: "/aql", label: "AQL Runner", icon: "Q" },
  { path: "/servers", label: "Servers", icon: "S" },
];

function isActive(path: string): boolean {
  return route.path.startsWith(path);
}
</script>

<template>
  <nav class="sidebar-nav">
    <router-link
      v-for="item in navItems"
      :key="item.path"
      :to="item.path"
      class="nav-item"
      :class="{ active: isActive(item.path) }"
    >
      <span class="nav-icon">{{ item.icon }}</span>
      <span class="nav-label">{{ item.label }}</span>
    </router-link>
  </nav>
</template>

<style scoped>
.sidebar-nav {
  display: flex;
  flex-direction: column;
  padding: 8px;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius);
  color: var(--color-text-secondary);
  text-decoration: none;
  font-size: 13px;
  transition: all 0.15s;
}
.nav-item:hover {
  background: var(--color-surface);
  color: var(--color-text);
}
.nav-item.active {
  background: var(--color-surface);
  color: var(--color-primary);
}

.nav-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: var(--color-bg-tertiary);
  font-size: 12px;
  font-weight: 700;
  font-family: var(--font-mono);
}
.nav-item.active .nav-icon {
  background: var(--color-primary-dim);
  color: #fff;
}
</style>
