import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import "./styles/medblocks-overrides.css";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/ehrs",
    },
    {
      path: "/ehrs",
      name: "ehrs",
      component: () => import("./views/EhrBrowser.vue"),
    },
    {
      path: "/ehrs/:ehrId",
      name: "ehr-detail",
      component: () => import("./views/EhrBrowser.vue"),
      props: true,
    },
    {
      path: "/ehrs/:ehrId/compositions/:compositionUid",
      name: "composition",
      component: () => import("./views/CompositionViewer.vue"),
      props: true,
    },
    {
      path: "/templates",
      name: "templates",
      component: () => import("./views/TemplateBrowser.vue"),
    },
    {
      path: "/templates/:templateId",
      name: "template-detail",
      component: () => import("./views/TemplateBrowser.vue"),
      props: true,
    },
    {
      path: "/aql",
      name: "aql",
      component: () => import("./views/AqlRunner.vue"),
    },
    {
      path: "/servers",
      name: "servers",
      component: () => import("./views/ServerManager.vue"),
    },
    {
      path: "/compose/:templateId",
      name: "compose",
      component: () => import("./views/CompositionForm.vue"),
      props: true,
    },
    {
      path: "/compose/:templateId/:ehrId",
      name: "compose-for-ehr",
      component: () => import("./views/CompositionForm.vue"),
      props: true,
    },
    {
      path: "/edit/:ehrId/:compositionUid",
      name: "edit-composition",
      component: () => import("./views/CompositionForm.vue"),
      props: true,
    },
  ],
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
