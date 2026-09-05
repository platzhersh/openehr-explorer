<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useSettingsStore } from "../stores/settings";
import { useTourStore } from "../stores/tour";
import { useAnalytics } from "../composables/useAnalytics";
import CopyButton from "../components/CopyButton.vue";
import TerminologySystemSelect from "../components/TerminologySystemSelect.vue";
import TourReplayButton from "../components/TourReplayButton.vue";
import {
  describeCode,
  expandValueSet,
  validateCode,
  testSubsumption,
  type CodeDescription,
  type ValueSetExpansion,
  type CodeValidation,
  type SubsumptionResult,
  type TerminologyConcept,
} from "../lib/terminology";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const settingsStore = useSettingsStore();
const tourStore = useTourStore();
const analytics = useAnalytics();

type TabId = "lookup" | "expand" | "validate" | "subsumes";

const TABS: { id: TabId; label: string; operation: string; blurb: string; docUrl: string }[] = [
  {
    id: "lookup",
    label: "Describe a Code",
    operation: "CodeSystem/$lookup",
    blurb:
      "Look up a single code and see its preferred term, synonyms, and properties (e.g. parent, inactive). Use this to check what a stored or bound code actually means.",
    docUrl: "https://www.hl7.org/fhir/codesystem-operation-lookup.html",
  },
  {
    id: "expand",
    label: "Expand a Value Set",
    operation: "ValueSet/$expand",
    blurb:
      "List the concepts that belong to a value set, optionally narrowed by a text filter. Use this to see the legal options for a coded field before you pick one.",
    docUrl: "https://www.hl7.org/fhir/valueset-operation-expand.html",
  },
  {
    id: "validate",
    label: "Validate Membership",
    operation: "ValueSet/$validate-code or CodeSystem/$validate-code",
    blurb:
      "Check whether a code is a legal member of a value set — or just a valid code in its code system, if you leave the value set blank.",
    docUrl: "https://www.hl7.org/fhir/valueset-operation-validate-code.html",
  },
  {
    id: "subsumes",
    label: "Test Subsumption",
    operation: "CodeSystem/$subsumes",
    blurb:
      "Compare two codes from the same code system and see how they relate in the hierarchy: equivalent, one broader than the other, or unrelated.",
    docUrl: "https://www.hl7.org/fhir/codesystem-operation-subsumes.html",
  },
];

const activeTab = ref<TabId>("lookup");
// `.find()` is statically typed as possibly `undefined`; falling back to
// TABS[0] (rather than a `!` non-null assertion) keeps this total even
// though activeTab, by construction, always matches one of TABS' ids.
const activeTabInfo = computed(() => TABS.find((t) => t.id === activeTab.value) ?? TABS[0]);

// Well-known FHIR core value sets, offered as one-click presets — remembering
// (or looking up) a canonical value set URL by hand is the main friction
// point of the Expand tab. Not exhaustive, just enough to get a first result
// without leaving the app; any other canonical URL still works by typing it.
const EXPAND_PRESETS = [
  { label: "Administrative Gender", url: "http://hl7.org/fhir/ValueSet/administrative-gender" },
  { label: "Marital Status", url: "http://hl7.org/fhir/ValueSet/marital-status" },
  { label: "Observation Status", url: "http://hl7.org/fhir/ValueSet/observation-status" },
  { label: "Condition Clinical Status", url: "http://hl7.org/fhir/ValueSet/condition-clinical" },
];

// A profile-level override always wins; falling back to the global default
// mirrors `effective_terminology_url` in settings.rs — this is purely for
// the empty-state hint below, the actual resolution happens server-side.
const effectiveTerminologyUrl = computed(
  () => serverStore.activeServer?.terminology_url || settingsStore.settings.terminology_server_url,
);

// SNOMED CT is the default system for every tab and the one used by every
// "Try an example" shortcut below — named once so it isn't repeated as a
// magic string at each call site.
const DEFAULT_SYSTEM = "SNOMED-CT";

// --- Describe a Code -------------------------------------------------
const lookupSystem = ref(DEFAULT_SYSTEM);
const lookupCodeInput = ref("");
const lookupLoading = ref(false);
const lookupError = ref<string | null>(null);
const lookupResult = ref<CodeDescription | null>(null);
const canLookup = computed(() => !!lookupSystem.value.trim() && !!lookupCodeInput.value.trim());

/** Fills in a known-good SNOMED CT code and runs it — a one-click way to see what this tab does. */
function runLookupExample() {
  lookupSystem.value = DEFAULT_SYSTEM;
  lookupCodeInput.value = "91302008";
  void runLookup();
}

async function runLookup() {
  if (!serverStore.activeServerId || !canLookup.value || lookupLoading.value) return;
  lookupLoading.value = true;
  lookupError.value = null;
  lookupResult.value = null;
  try {
    lookupResult.value = await describeCode(
      serverStore.activeServerId,
      lookupSystem.value.trim(),
      lookupCodeInput.value.trim(),
    );
    void analytics.track("terminology_query_run", { operation: "lookup" });
  } catch (e) {
    lookupError.value = String(e);
  } finally {
    lookupLoading.value = false;
  }
}

// --- Expand a Value Set ------------------------------------------------
const expandUrl = ref("");
const expandFilter = ref("");
const expandCount = ref(100);
const expandLoading = ref(false);
const expandError = ref<string | null>(null);
const expandResult = ref<ValueSetExpansion | null>(null);
const canExpand = computed(() => !!expandUrl.value.trim());

/** Fills in one of the well-known value set presets and runs it immediately. */
function applyExpandPreset(url: string) {
  expandUrl.value = url;
  void runExpand();
}

async function runExpand() {
  if (!serverStore.activeServerId || !canExpand.value || expandLoading.value) return;
  expandLoading.value = true;
  expandError.value = null;
  expandResult.value = null;
  try {
    expandResult.value = await expandValueSet(
      serverStore.activeServerId,
      expandUrl.value.trim(),
      expandFilter.value.trim() || undefined,
      expandCount.value || undefined,
    );
    void analytics.track("terminology_query_run", { operation: "expand" });
  } catch (e) {
    expandError.value = String(e);
  } finally {
    expandLoading.value = false;
  }
}

// --- Validate Membership -------------------------------------------------
const validateSystem = ref(DEFAULT_SYSTEM);
const validateCodeInput = ref("");
const validateValueSetUrl = ref("");
const validateLoading = ref(false);
const validateError = ref<string | null>(null);
const validateResult = ref<CodeValidation | null>(null);
const canValidate = computed(
  () => !!validateSystem.value.trim() && !!validateCodeInput.value.trim(),
);

/** Fills in a known-good SNOMED CT code and runs it — a one-click way to see what this tab does. */
function runValidateExample() {
  validateSystem.value = DEFAULT_SYSTEM;
  validateCodeInput.value = "386661006";
  validateValueSetUrl.value = "";
  void runValidate();
}

async function runValidate() {
  if (!serverStore.activeServerId || !canValidate.value || validateLoading.value) return;
  validateLoading.value = true;
  validateError.value = null;
  validateResult.value = null;
  try {
    validateResult.value = await validateCode(
      serverStore.activeServerId,
      validateSystem.value.trim(),
      validateCodeInput.value.trim(),
      validateValueSetUrl.value.trim() || undefined,
    );
    void analytics.track("terminology_query_run", { operation: "validate" });
  } catch (e) {
    validateError.value = String(e);
  } finally {
    validateLoading.value = false;
  }
}

// --- Test Subsumption -------------------------------------------------
const subsumesSystem = ref(DEFAULT_SYSTEM);
const subsumesCodeA = ref("");
const subsumesCodeB = ref("");
const subsumesLoading = ref(false);
const subsumesError = ref<string | null>(null);
const subsumesResult = ref<SubsumptionResult | null>(null);
const canSubsume = computed(
  () =>
    !!subsumesSystem.value.trim() && !!subsumesCodeA.value.trim() && !!subsumesCodeB.value.trim(),
);

/** Fills in two related SNOMED CT codes and runs it — a one-click way to see what this tab does. */
function runSubsumesExample() {
  subsumesSystem.value = DEFAULT_SYSTEM;
  subsumesCodeA.value = "64572001";
  subsumesCodeB.value = "195967001";
  void runSubsumes();
}

const SUBSUMPTION_EXPLANATIONS: Record<string, string> = {
  equivalent: "Code A and Code B are the same concept.",
  subsumes: "Code A is a broader concept that includes Code B.",
  "subsumed-by": "Code A is a narrower concept, included in Code B.",
  "not-subsumed": "Neither code subsumes the other.",
};

async function runSubsumes() {
  if (!serverStore.activeServerId || !canSubsume.value || subsumesLoading.value) return;
  subsumesLoading.value = true;
  subsumesError.value = null;
  subsumesResult.value = null;
  try {
    subsumesResult.value = await testSubsumption(
      serverStore.activeServerId,
      subsumesSystem.value.trim(),
      subsumesCodeA.value.trim(),
      subsumesCodeB.value.trim(),
    );
    void analytics.track("terminology_query_run", { operation: "subsumes" });
  } catch (e) {
    subsumesError.value = String(e);
  } finally {
    subsumesLoading.value = false;
  }
}

// Deep-link support: a `system`/`code` query param (e.g. from a template's
// "Bound Concepts" panel — see TemplateBrowser.vue) pre-fills and runs the
// Describe tab immediately, so following the link lands on an answer, not
// just a pre-filled form.
function applyRouteQuery() {
  const system = route.query.system;
  const code = route.query.code;
  if (typeof system === "string" && typeof code === "string" && system && code) {
    activeTab.value = "lookup";
    lookupSystem.value = system;
    lookupCodeInput.value = code;
    void runLookup();
  }
}

watch(() => route.query, applyRouteQuery, { immediate: true });

// The "Try an example" tour step (`[data-tour="terminology-example"]`) only
// exists on the Describe tab's markup — force that tab active whenever this
// view's tour starts (auto or via the compass icon), so the step's target
// is there regardless of which tab the user happened to be on.
watch(
  () => tourStore.activeTourId,
  (id) => {
    if (id === "terminology") activeTab.value = "lookup";
  },
);

// The "Describe →" link from a template's Bound Concepts panel also carries
// `fromTemplate` (the template that sent us here) — used to show a "Back to
// template" link, so following a term binding out to the Terminology
// Browser doesn't strand the user with no way back except re-navigating by
// hand. Absent when Terminology was opened directly (e.g. from the sidebar),
// so the link only appears when there's actually somewhere to go back to.
const fromTemplateId = computed(() => {
  const id = route.query.fromTemplate;
  return typeof id === "string" && id ? id : null;
});

function goBackToTemplate() {
  if (!fromTemplateId.value) return;
  router.push({ name: "template-detail", params: { templateId: fromTemplateId.value } });
}

function useConceptInLookup(concept: TerminologyConcept) {
  // Every concept a `$expand` returns is required by the FHIR spec to carry
  // its own `system` — falling back to the value set's own URL here (as an
  // earlier version of this code did) would send that URL to `$lookup` as
  // if it were a code system, which is a different resource. If a
  // non-conformant server ever omits it, better to leave the previously
  // entered system in place than substitute something known-wrong.
  if (!concept.code) return;
  activeTab.value = "lookup";
  if (concept.system) lookupSystem.value = concept.system;
  lookupCodeInput.value = concept.code;
  void runLookup();
}
</script>

<template>
  <div class="terminology-browser">
    <div class="panel-header">
      <div>
        <button
          v-if="fromTemplateId"
          type="button"
          class="btn btn-sm back-to-template"
          @click="goBackToTemplate"
        >
          ← Back to template
        </button>
        <div class="header-title-row">
          <h2>Terminology</h2>
          <TourReplayButton
            v-if="serverStore.activeServerId && effectiveTerminologyUrl"
            tour-id="terminology"
            view-label="Terminology Browser"
          />
        </div>
        <p class="subtitle">
          Query the FHIR terminology server configured for this connection: describe a code, expand
          a value set, and test membership or subsumption.
        </p>
        <p class="scope-note" data-tour="terminology-scope-note">
          <strong>Not part of openEHR itself</strong> — this talks directly to a
          <a href="https://www.hl7.org/fhir/terminology-service.html" target="_blank" rel="noopener"
            >FHIR Terminology Service</a
          >, a separate HL7 standard for code systems and value sets. openEHR doesn't define
          terminology operations of its own; the app just calls the FHIR server configured in
          <router-link to="/settings">Settings</router-link>
          or on the server profile, independently of your openEHR CDR.
        </p>
      </div>
    </div>

    <div v-if="!serverStore.activeServerId" class="empty-state">
      <h3>No server selected</h3>
      <p>Select a server to query its configured terminology server.</p>
    </div>

    <div v-else-if="!effectiveTerminologyUrl" class="empty-state terminology-disabled">
      <h3>No terminology server configured</h3>
      <p>
        Set a default terminology server URL in
        <router-link to="/settings">Settings</router-link>, or override it per-server in
        <router-link to="/servers">Server Manager</router-link>, to browse terminologies here.
      </p>
    </div>

    <template v-else>
      <div class="tab-bar" role="tablist" data-tour="terminology-tabs">
        <button
          v-for="tab in TABS"
          :key="tab.id"
          type="button"
          role="tab"
          class="tab"
          :class="{ active: activeTab === tab.id }"
          :aria-selected="activeTab === tab.id"
          @click="activeTab = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>

      <div class="tab-blurb">
        <p>
          <span class="operation-name">{{ activeTabInfo.operation }}</span> —
          {{ activeTabInfo.blurb }}
          <a :href="activeTabInfo.docUrl" target="_blank" rel="noopener" class="doc-link"
            >FHIR spec ↗</a
          >
        </p>
      </div>

      <!-- Describe a Code -->
      <div v-if="activeTab === 'lookup'" class="tool-panel">
        <form class="tool-form" @submit.prevent="runLookup">
          <div class="form-row">
            <TerminologySystemSelect v-model="lookupSystem" label="Terminology system" />
            <label>
              Code
              <input class="input" v-model="lookupCodeInput" placeholder="91302008" />
            </label>
            <button type="submit" class="btn btn-primary" :disabled="lookupLoading || !canLookup">
              {{ lookupLoading ? "Looking up…" : "Describe" }}
            </button>
            <button
              type="button"
              class="btn btn-sm btn-ghost"
              data-tour="terminology-example"
              :disabled="lookupLoading"
              @click="runLookupExample"
            >
              Try an example
            </button>
          </div>
        </form>

        <div v-if="lookupError" class="error-msg">{{ lookupError }}</div>

        <div v-else-if="lookupResult" class="result-card">
          <div class="result-header">
            <span class="result-display">{{ lookupResult.display || "(no display term)" }}</span>
            <CopyButton :text="lookupResult.code" title="Copy code" />
          </div>
          <div class="result-meta">
            <span class="badge">{{ lookupResult.system }}</span>
            <span class="term-code">{{ lookupResult.code }}</span>
          </div>

          <div v-if="lookupResult.designations.length" class="result-section">
            <h4>Designations</h4>
            <ul class="designation-list">
              <li v-for="(d, i) in lookupResult.designations" :key="i">{{ d }}</li>
            </ul>
          </div>

          <div v-if="lookupResult.properties.length" class="result-section">
            <h4>Properties</h4>
            <table class="result-table">
              <thead>
                <tr>
                  <th>Property</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(p, i) in lookupResult.properties" :key="i">
                  <td class="prop-code">{{ p.code }}</td>
                  <td>{{ p.value }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Expand a Value Set -->
      <div v-if="activeTab === 'expand'" class="tool-panel">
        <form class="tool-form" @submit.prevent="runExpand">
          <div class="form-row">
            <label class="grow">
              Value set URL
              <input
                class="input"
                v-model="expandUrl"
                placeholder="http://hl7.org/fhir/ValueSet/administrative-gender"
              />
            </label>
            <label>
              Filter
              <input class="input" v-model="expandFilter" placeholder="optional text filter" />
            </label>
            <label class="count-field">
              Count
              <input class="input" type="number" min="1" max="1000" v-model.number="expandCount" />
            </label>
            <button type="submit" class="btn btn-primary" :disabled="expandLoading || !canExpand">
              {{ expandLoading ? "Expanding…" : "Expand" }}
            </button>
          </div>
          <p class="form-help">
            A canonical value set URL — e.g. a FHIR core value set, or one bound to a template
            node's term binding (see a template's "Bound Concepts" panel). Try one of:
          </p>
          <div class="preset-chips">
            <button
              v-for="preset in EXPAND_PRESETS"
              :key="preset.url"
              type="button"
              class="btn btn-sm btn-ghost"
              :disabled="expandLoading"
              @click="applyExpandPreset(preset.url)"
            >
              {{ preset.label }}
            </button>
          </div>
        </form>

        <div v-if="expandError" class="error-msg">{{ expandError }}</div>

        <div v-else-if="expandResult" class="result-card">
          <div class="result-meta">
            <span class="badge"
              >{{ expandResult.concepts.length }} shown{{
                expandResult.total != null ? ` of ${expandResult.total}` : ""
              }}</span
            >
          </div>
          <table v-if="expandResult.concepts.length" class="result-table concept-table">
            <thead>
              <tr>
                <th>Code</th>
                <th>Display</th>
                <th>System</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(c, i) in expandResult.concepts" :key="i">
                <td class="term-code">{{ c.code }}</td>
                <td>{{ c.display }}</td>
                <td class="system-cell">{{ c.system }}</td>
                <td>
                  <button type="button" class="btn btn-sm" @click="useConceptInLookup(c)">
                    Describe
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <p v-else class="empty-hint">This value set expanded to no members.</p>
        </div>
      </div>

      <!-- Validate Membership -->
      <div v-if="activeTab === 'validate'" class="tool-panel">
        <form class="tool-form" @submit.prevent="runValidate">
          <div class="form-row">
            <TerminologySystemSelect v-model="validateSystem" label="Terminology system" />
            <label>
              Code
              <input class="input" v-model="validateCodeInput" placeholder="386661006" />
            </label>
            <label class="grow">
              Value set URL (optional)
              <input
                class="input"
                v-model="validateValueSetUrl"
                placeholder="leave empty to validate against the code system itself"
              />
            </label>
            <button
              type="submit"
              class="btn btn-primary"
              :disabled="validateLoading || !canValidate"
            >
              {{ validateLoading ? "Validating…" : "Validate" }}
            </button>
            <button
              type="button"
              class="btn btn-sm btn-ghost"
              :disabled="validateLoading"
              @click="runValidateExample"
            >
              Try an example
            </button>
          </div>
        </form>

        <div v-if="validateError" class="error-msg">{{ validateError }}</div>

        <div v-else-if="validateResult" class="result-card">
          <div class="result-header">
            <span
              class="badge outcome-badge"
              :class="validateResult.result ? 'outcome-valid' : 'outcome-invalid'"
            >
              {{ validateResult.result ? "VALID" : "INVALID" }}
            </span>
            <span v-if="validateResult.display" class="result-display">{{
              validateResult.display
            }}</span>
          </div>
          <p v-if="validateResult.message" class="result-message">{{ validateResult.message }}</p>
        </div>
      </div>

      <!-- Test Subsumption -->
      <div v-if="activeTab === 'subsumes'" class="tool-panel">
        <form class="tool-form" @submit.prevent="runSubsumes">
          <div class="form-row">
            <TerminologySystemSelect v-model="subsumesSystem" label="Terminology system" />
            <label>
              Code A
              <input class="input" v-model="subsumesCodeA" placeholder="64572001" />
            </label>
            <label>
              Code B
              <input class="input" v-model="subsumesCodeB" placeholder="195967001" />
            </label>
            <button
              type="submit"
              class="btn btn-primary"
              :disabled="subsumesLoading || !canSubsume"
            >
              {{ subsumesLoading ? "Testing…" : "Test" }}
            </button>
            <button
              type="button"
              class="btn btn-sm btn-ghost"
              :disabled="subsumesLoading"
              @click="runSubsumesExample"
            >
              Try an example
            </button>
          </div>
        </form>

        <div v-if="subsumesError" class="error-msg">{{ subsumesError }}</div>

        <div v-else-if="subsumesResult" class="result-card">
          <span class="badge outcome-badge">{{ subsumesResult.outcome }}</span>
          <p class="result-message">
            {{ SUBSUMPTION_EXPLANATIONS[subsumesResult.outcome] || "Unrecognised outcome." }}
          </p>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.terminology-browser {
  padding: 0 24px 24px;
  max-width: 960px;
}

.panel-header {
  padding: 16px 0;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 20px;
}
.panel-header h2 {
  font-size: 16px;
  font-weight: 600;
}
.back-to-template {
  display: block;
  margin-bottom: 8px;
}
.header-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.subtitle {
  margin-top: 4px;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.scope-note {
  margin-top: 8px;
  font-size: 12px;
  color: var(--color-text-muted);
  line-height: 1.5;
}
.scope-note a {
  color: var(--color-primary);
  text-decoration: none;
}
.scope-note a:hover {
  text-decoration: underline;
}

.terminology-disabled a {
  color: var(--color-primary);
}

.tab-bar {
  display: flex;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  overflow: hidden;
  width: fit-content;
  margin-bottom: 12px;
}
.tab {
  padding: 6px 14px;
  background: var(--color-surface);
  color: var(--color-text-secondary);
  border: none;
  font-size: 13px;
  cursor: pointer;
}
.tab:not(:last-child) {
  border-right: 1px solid var(--color-border);
}
.tab.active {
  background: var(--color-primary-dim);
  color: #fff;
}

.tab-blurb {
  margin-bottom: 20px;
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.5;
}
.operation-name {
  font-family: var(--font-mono);
  color: var(--color-primary);
}
.doc-link {
  margin-left: 4px;
  color: var(--color-primary);
  text-decoration: none;
  white-space: nowrap;
}
.doc-link:hover {
  text-decoration: underline;
}

.tool-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tool-form {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 12px;
}

.form-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  flex-wrap: wrap;
}
.form-row label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--color-text-secondary);
}
.form-row label.grow {
  flex: 1;
  min-width: 220px;
}
.count-field {
  width: 90px;
}

.form-help {
  margin-top: 10px;
  font-size: 12px;
  color: var(--color-text-muted);
  line-height: 1.4;
}
.preset-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}
.count-field .input {
  width: 100%;
}

.result-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 16px;
}

.result-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.result-display {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text);
}

.result-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  font-size: 12px;
}
.term-code {
  font-family: var(--font-mono);
  color: var(--color-primary);
  font-weight: 500;
}

.result-section {
  margin-top: 12px;
}
.result-section h4 {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 6px;
}

.designation-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
.designation-list li::before {
  content: "— ";
  color: var(--color-text-muted);
}

.result-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.result-table td,
.result-table th {
  padding: 6px 8px;
  border-bottom: 1px solid var(--color-border);
  text-align: left;
}
.result-table th {
  color: var(--color-text-secondary);
  font-weight: 600;
}
.prop-code {
  font-family: var(--font-mono);
  color: var(--color-text-secondary);
  white-space: nowrap;
}
.system-cell {
  color: var(--color-text-muted);
  font-size: 11px;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.concept-table td {
  vertical-align: middle;
}

.outcome-badge {
  font-weight: 700;
  letter-spacing: 0.3px;
  text-transform: uppercase;
  font-size: 11px;
}
.outcome-valid {
  background: rgba(107, 255, 142, 0.12);
  color: var(--color-success);
  border-color: rgba(107, 255, 142, 0.3);
}
.outcome-invalid {
  background: rgba(255, 107, 107, 0.12);
  color: var(--color-error);
  border-color: rgba(255, 107, 107, 0.3);
}

.result-message {
  margin-top: 8px;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.empty-hint {
  color: var(--color-text-muted);
  font-size: 13px;
}
</style>
