<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div
    class="panelcontainer"
    @mouseover="() => (isCurrentlyHoveredPanel = true)"
    @mouseleave="() => (isCurrentlyHoveredPanel = false)"
    data-test="dashboard-panel-container"
  >
    <div class="drag-allow">
      <q-bar
        :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
        dense
        class="q-px-xs"
        style="border-top-left-radius: 3px; border-top-right-radius: 3px"
        data-test="dashboard-panel-bar"
      >
        <q-icon
          v-if="!viewOnly"
          name="drag_indicator"
          data-test="dashboard-panel-drag"
        />
        <div :title="props.data.title" class="panelHeader">
          {{ props.data.title }}
        </div>
        <q-space />
        <q-icon
          v-if="
            !viewOnly && isCurrentlyHoveredPanel && props.data.description != ''
          "
          name="info_outline"
          style="cursor: pointer"
          data-test="dashboard-panel-description-info"
        >
          <q-tooltip anchor="bottom right" self="top right" max-width="220px">
            <div style="white-space: pre-wrap">
              {{ props.data.description }}
            </div>
          </q-tooltip>
        </q-icon>
        <q-btn
          v-if="!viewOnly && isCurrentlyHoveredPanel"
          icon="fullscreen"
          flat
          size="sm"
          padding="1px"
          @click="onPanelModifyClick('ViewPanel')"
          title="Full screen"
          data-test="dashboard-panel-fullscreen-btn"
        />
        <q-btn
          v-if="dependentAdHocVariable"
          :icon="outlinedWarning"
          flat
          size="xs"
          padding="2px"
          @click="showViewPanel = true"
          data-test="dashboard-panel-dependent-adhoc-variable-btn"
        >
          <q-tooltip anchor="bottom right" self="top right" max-width="220px">
            Some dynamic variables are not applied because the field is not
            present in the query's stream. Open Query Inspector to see all the
            details of the variables and queries executed to render this panel
          </q-tooltip>
        </q-btn>
        <!-- show error here -->
        <q-btn
          v-if="errorData"
          :key="errorData"
          :icon="outlinedWarning"
          flat
          size="xs"
          padding="2px"
          data-test="dashboard-panel-error-data"
          class="warning"
        >
          <q-tooltip anchor="bottom right" self="top right" max-width="220px">
            <div style="white-space: pre-wrap">
              {{ errorData }}
            </div>
          </q-tooltip>
        </q-btn>
        <q-btn
          v-if="maxQueryRange.length > 0"
          :icon="outlinedWarning"
          flat
          size="xs"
          padding="2px"
          data-test="dashboard-panel-max-duration-warning"
          class="warning"
        >
          <q-tooltip anchor="bottom right" self="top right" max-width="220px">
            <div style="white-space: pre-wrap">
              {{ maxQueryRange.join("\n\n") }}
            </div>
          </q-tooltip>
        </q-btn>
        <q-btn
          v-if="limitNumberOfSeriesWarningMessage"
          :icon="symOutlinedDataInfoAlert"
          flat
          size="xs"
          padding="2px"
          data-test="dashboard-panel-limit-number-of-series-warning"
          class="warning"
        >
          <q-tooltip anchor="bottom right" self="top right">
            <div style="white-space: pre-wrap">
              {{ limitNumberOfSeriesWarningMessage }}
            </div>
          </q-tooltip>
        </q-btn>
        <q-btn
          v-if="isCachedDataDifferWithCurrentTimeRange"
          :icon="outlinedRunningWithErrors"
          flat
          size="xs"
          padding="2px"
          data-test="dashboard-panel-is-cached-data-differ-with-current-time-range-warning"
        >
          <q-tooltip anchor="bottom right" self="top right">
            <div style="white-space: pre-wrap">
              The data shown is cached and is different from the selected time
              range.
            </div>
          </q-tooltip>
        </q-btn>
        <q-btn
          v-if="isPartialData && !isPanelLoading"
          :icon="symOutlinedClockLoader20"
          flat
          size="xs"
          padding="2px"
          data-test="dashboard-panel-partial-data-warning"
          class="warning"
        >
          <q-tooltip anchor="bottom right" self="top right">
            <div style="white-space: pre-wrap">
              The data shown is incomplete because the loading was interrupted.
              Refresh to load complete data.
            </div>
          </q-tooltip>
        </q-btn>
        <span v-if="lastTriggeredAt && !viewOnly" class="lastRefreshedAt">
          <span class="lastRefreshedAtIcon"
            >🕑
            <q-tooltip anchor="bottom right" self="top right">
              Last Refreshed: <RelativeTime :timestamp="lastTriggeredAt" />
            </q-tooltip>
          </span>
          <RelativeTime
            :timestamp="lastTriggeredAt"
            fullTimePrefix="Last Refreshed At: "
          />
        </span>
        <q-btn
          v-if="!viewOnly"
          icon="refresh"
          flat
          size="sm"
          padding="1px"
          @click="onRefreshPanel"
          title="Refresh Panel"
          data-test="dashboard-panel-refresh-panel-btn"
          :color="variablesDataUpdated ? 'warning' : ''"
          :disable="isPanelLoading"
        >
          <q-tooltip>
            {{
              variablesDataUpdated
                ? "Refresh to apply latest variable changes"
                : "Refresh"
            }}
          </q-tooltip>
        </q-btn>
        <q-btn-dropdown
          :data-test="`dashboard-edit-panel-${props.data.title}-dropdown`"
          dense
          flat
          label=""
          no-caps
          v-if="!viewOnly"
        >
          <q-list dense>
            <q-item
              clickable
              v-close-popup="true"
              @click="onPanelModifyClick('EditPanel')"
            >
              <q-item-section>
                <q-item-label data-test="dashboard-edit-panel" class="q-pa-sm"
                  >Edit Panel</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-close-popup="true"
              @click="onPanelModifyClick('EditLayout')"
            >
              <q-item-section>
                <q-item-label data-test="dashboard-edit-layout" class="q-pa-sm"
                  >Edit Layout</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-close-popup="true"
              @click="onPanelModifyClick('DuplicatePanel')"
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-duplicate-panel"
                  class="q-pa-sm"
                  >Duplicate</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-close-popup="true"
              @click="onPanelModifyClick('DeletePanel')"
            >
              <q-item-section>
                <q-item-label data-test="dashboard-delete-panel" class="q-pa-sm"
                  >Delete Panel</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-if="metaData && metaData.queries?.length > 0"
              v-close-popup="true"
              @click="showViewPanel = true"
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-query-inspector-panel"
                  class="q-pa-sm"
                  >Query Inspector</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-if="metaData && metaData.queries?.length > 0"
              v-close-popup="true"
              @click="
                PanleSchemaRendererRef?.downloadDataAsCSV(props.data.title)
              "
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-panel-download-as-csv-btn"
                  class="q-pa-sm"
                  >Download as CSV</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-if="metaData && metaData.queries?.length > 0"
              v-close-popup="true"
              @click="
                PanleSchemaRendererRef?.downloadDataAsJSON(props.data.title)
              "
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-panel-download-as-json-btn"
                  class="q-pa-sm"
                  >Download as JSON</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-if="metaData && metaData.queries?.length > 0"
              :disable="props.data.queryType != 'sql'"
              v-close-popup="true"
              @click="onLogPanel"
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-move-to-logs-module"
                  class="q-pa-sm"
                  >Go To Logs</q-item-label
                >
              </q-item-section>
            </q-item>
            <q-item
              clickable
              v-close-popup="true"
              @click="onPanelModifyClick('MovePanel')"
            >
              <q-item-section>
                <q-item-label
                  data-test="dashboard-move-to-another-panel"
                  class="q-pa-sm"
                  >Move To Another Tab</q-item-label
                >
              </q-item-section>
            </q-item>
          </q-list>
        </q-btn-dropdown>
      </q-bar>
    </div>
    <PanelSchemaRenderer
      :panelSchema="props.data"
      :selectedTimeObj="props.selectedTimeDate"
      :width="props.width"
      :height="props.height"
      :variablesData="props.variablesData"
      :forceLoad="props.forceLoad"
      :searchType="searchType"
      :dashboard-id="props.dashboardId"
      :folder-id="props.folderId"
      :report-id="props.reportId"
      @loading-state-change="handleLoadingStateChange"
      @metadata-update="metaDataValue"
      @limit-number-of-series-warning-message-update="
        handleLimitNumberOfSeriesWarningMessageUpdate
      "
      @result-metadata-update="handleResultMetadataUpdate"
      @last-triggered-at-update="handleLastTriggeredAtUpdate"
      @is-cached-data-differ-with-current-time-range-update="
        handleIsCachedDataDifferWithCurrentTimeRangeUpdate
      "
      @updated:data-zoom="$emit('updated:data-zoom', $event)"
      @update:initial-variable-values="
        (...args) => $emit('update:initial-variable-values', ...args)
      "
      @error="onError"
      @is-partial-data-update="handleIsPartialDataUpdate"
      ref="PanleSchemaRendererRef"
      :allowAnnotationsAdd="true"
    ></PanelSchemaRenderer>
    <q-dialog v-model="showViewPanel">
      <QueryInspector :metaData="metaData" :data="props.data"></QueryInspector>
    </q-dialog>

    <ConfirmDialog
      title="Delete Panel"
      message="Are you sure you want to delete this Panel?"
      @update:ok="deletePanelDialog"
      @update:cancel="confirmDeletePanelDialog = false"
      v-model="confirmDeletePanelDialog"
    />

    <SinglePanelMove
      title="Move Panel to Another Tab"
      message="Select destination tab"
      @update:ok="movePanelDialog"
      :key="confirmMovePanelDialog"
      @update:cancel="confirmMovePanelDialog = false"
      v-model="confirmMovePanelDialog"
      @refresh="$emit('refresh')"
    />
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  computed,
  defineAsyncComponent,
  watch,
  onBeforeUnmount,
} from "vue";
import PanelSchemaRenderer from "./PanelSchemaRenderer.vue";
import { useStore } from "vuex";
import { useRoute, useRouter } from "vue-router";
import { addPanel } from "@/utils/commons";
import { useQuasar } from "quasar";
import ConfirmDialog from "../ConfirmDialog.vue";
import {
  outlinedWarning,
  outlinedRunningWithErrors,
} from "@quasar/extras/material-icons-outlined";
import {
  symOutlinedClockLoader20,
  symOutlinedDataInfoAlert,
} from "@quasar/extras/material-symbols-outlined";
import SinglePanelMove from "@/components/dashboards/settings/SinglePanelMove.vue";
import RelativeTime from "@/components/common/RelativeTime.vue";
import { getFunctionErrorMessage } from "@/utils/zincutils";
import useNotifications from "@/composables/useNotifications";
import { isEqual } from "lodash-es";
import { b64EncodeUnicode } from "@/utils/zincutils";
import shortURL from "@/services/short_url";

const QueryInspector = defineAsyncComponent(() => {
  return import("@/components/dashboards/QueryInspector.vue");
});

export default defineComponent({
  name: "PanelContainer",
  emits: [
    "onDeletePanel",
    "onViewPanel",
    "updated:data-zoom",
    "onMovePanel",
    "refreshPanelRequest",
    "refresh",
    "update:initial-variable-values",
    "onEditLayout",
  ],
  props: [
    "data",
    "selectedTimeDate",
    "viewOnly",
    "width",
    "height",
    "variablesData",
    "dashboardId",
    "metaData",
    "forceLoad",
    "searchType",
    "folderId",
    "reportId",
    "currentVariablesData",
  ],
  components: {
    PanelSchemaRenderer,
    QueryInspector,
    ConfirmDialog,
    SinglePanelMove,
    RelativeTime,
  },
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const route = useRoute();
    const $q = useQuasar();
    const metaData = ref();
    const showViewPanel = ref(false);
    const confirmDeletePanelDialog = ref(false);
    const confirmMovePanelDialog: any = ref(false);
    const {
      showPositiveNotification,
      showErrorNotification,
      showConfictErrorNotificationWithRefreshBtn,
    } = useNotifications();
    const metaDataValue = (metadata: any) => {
      metaData.value = metadata;
    };

    const maxQueryRange: any = ref([]);

    const limitNumberOfSeriesWarningMessage = ref("");

    const handleResultMetadataUpdate = (metadata: any) => {
      const combinedWarnings: any[] = [];
      metadata.forEach((query: any) => {
        if (
          query?.function_error &&
          query?.new_start_time &&
          query?.new_end_time
        ) {
          const combinedMessage = getFunctionErrorMessage(
            query.function_error,
            query.new_start_time,
            query.new_end_time,
            store.state.timezone,
          );
          combinedWarnings.push(combinedMessage);
        } else if (query?.function_error) {
          combinedWarnings.push(query.function_error);
        }
      });

      // NOTE: for multi query, just show the first query warning
      maxQueryRange.value =
        combinedWarnings.length > 0 ? [combinedWarnings[0]] : [];
    };

    // to store and show when the panel was last loaded
    const lastTriggeredAt = ref(null);
    const handleLastTriggeredAtUpdate = (data: any) => {
      lastTriggeredAt.value = data;
    };

    // to store and show warning if the cached data is different with current time range
    const isCachedDataDifferWithCurrentTimeRange: any = ref(false);
    const handleIsCachedDataDifferWithCurrentTimeRangeUpdate = (
      isDiffer: boolean,
    ) => {
      isCachedDataDifferWithCurrentTimeRange.value = isDiffer;
    };

    const handleLimitNumberOfSeriesWarningMessageUpdate = (message: string) => {
      limitNumberOfSeriesWarningMessage.value = message;
    };

    const showText = ref(false);

    // need PanleSchemaRendererRef for table download as a csv
    const PanleSchemaRendererRef: any = ref(null);

    //check if dependent adhoc variable exists
    const dependentAdHocVariable = computed(() => {
      if (!metaData.value) {
        return false;
      }

      const adhocVariables = props.variablesData.values
        ?.filter((it: any) => it.type === "dynamic_filters")
        ?.map((it: any) => it?.value)
        .flat()
        ?.filter((it: any) => it?.operator && it?.name && it?.value);

      const metaDataDynamic = metaData.value?.queries?.every((it: any) => {
        const vars = it?.variables?.filter(
          (it: any) => it.type === "dynamicVariable",
        );
        return vars?.length == adhocVariables?.length;
      });

      if (adhocVariables?.length == 0 || adhocVariables == undefined) {
        return false;
      }
      return !metaDataDynamic;
    });

    // for full screen button
    const isCurrentlyHoveredPanel: any = ref(false);

    //for edit panel
    const onEditPanel = (data: any) => {
      return router.push({
        path: "/dashboards/add_panel",
        query: {
          ...route.query,
          dashboard: String(route.query.dashboard),
          panelId: data.id,
          folder: route.query.folder ?? "default",
          tab: route.query.tab ?? data.panels[0]?.tabId,
        },
      });
    };
    const getOriginalQueryAndStream = (queryDetails: any, metadata: any) => {
      const originalQuery = metadata?.value?.queries[0]?.query;
      const streamName = queryDetails?.queries[0]?.fields?.stream;

      if (!originalQuery || !streamName) {
        console.error("Missing query or stream name.");
        return null;
      }

      return { originalQuery, streamName };
    };
    const constructLogsUrl = (
      streamName: string,
      encodedQuery: string,
      queryDetails: any,
      currentUrl: string,
      vrlFunctionQueryEncoded: string,
    ) => {
      const logsUrl = new URL(currentUrl + "/logs");
      logsUrl.searchParams.set(
        "stream_type",
        queryDetails.queries[0]?.fields?.stream_type,
      );
      logsUrl.searchParams.set("stream", streamName);
      logsUrl.searchParams.set(
        "from",
        metaData.value.queries[0]?.startTime.toString(),
      );
      logsUrl.searchParams.set(
        "to",
        metaData.value.queries[0]?.endTime.toString(),
      );
      logsUrl.searchParams.set("functionContent", vrlFunctionQueryEncoded);
      logsUrl.searchParams.set("sql_mode", "true");
      logsUrl.searchParams.set("query", encodedQuery);
      logsUrl.searchParams.set(
        "org_identifier",
        store.state.selectedOrganization.identifier,
      );
      logsUrl.searchParams.set("quick_mode", "false");
      logsUrl.searchParams.set("show_histogram", "false");

      return logsUrl;
    };

    const onLogPanel = async () => {
      const showNotification = showPositiveNotification(
        "Redirecting to logs page",
        {
          color: "warning",
        },
      );
      const queryDetails = props.data;
      if (!queryDetails) {
        console.error("Data is undefined.");
        return;
      }

      const { originalQuery, streamName } =
        getOriginalQueryAndStream(queryDetails, metaData) || {};
      if (!originalQuery || !streamName) return;

      let modifiedQuery = originalQuery;

      modifiedQuery = modifiedQuery.replace(/`/g, '"');

      const encodedQuery: any = b64EncodeUnicode(modifiedQuery);

      const vrlFunctionQuery = queryDetails.queries[0]?.vrlFunctionQuery;
      const vrlFunctionQueryEncoded: any = b64EncodeUnicode(vrlFunctionQuery);

      const pos = window.location.pathname.indexOf("/web/");
      const currentUrl =
        pos > -1
          ? window.location.origin +
            window.location.pathname.slice(0, pos) +
            "/web"
          : window.location.origin;

      const logsUrl = constructLogsUrl(
        streamName,
        encodedQuery,
        queryDetails,
        currentUrl,
        vrlFunctionQueryEncoded,
      );

      // Use short_url service to shorten the URL and redirect
      try {
        const org_identifier = store.state.selectedOrganization.identifier;
        const response = await shortURL.create(
          org_identifier,
          logsUrl.toString(),
        );
        const shortUrl = response?.data?.short_url;
        if (shortUrl) {
          window.open(shortUrl, "_blank");
        } else {
          window.open(logsUrl.toString(), "_blank");
        }
        showNotification();
      } catch (error) {
        window.open(logsUrl.toString(), "_blank");
      }
    };

    //create a duplicate panel
    const onDuplicatePanel = async (data: any): Promise<void> => {
      // Show a loading spinner notification.
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      // Generate a unique panel ID.
      const panelId =
        "Panel_ID" + Math.floor(Math.random() * (99999 - 10 + 1)) + 10;

      // Duplicate the panel data with the new ID.
      const panelData = JSON.parse(JSON.stringify(data));
      panelData.id = panelId;

      try {
        // Add the duplicated panel to the dashboard.
        await addPanel(
          store,
          route.query.dashboard,
          panelData,
          route.query.folder ?? "default",
          route.query.tab ?? data.panels[0]?.tabId,
        );

        // Show a success notification.
        showPositiveNotification("Panel Duplicated Successfully");

        // Navigate to the new panel.
        router.push({
          path: "/dashboards/add_panel",
          query: {
            ...route.query,
            dashboard: String(route.query.dashboard),
            panelId: panelId,
            folder: route.query.folder ?? "default",
            tab: route.query.tab ?? data.panels[0]?.tabId,
          },
        });
        return;
      } catch (error: any) {
        // Show an error notification.

        if (error?.response?.status === 409) {
          showConfictErrorNotificationWithRefreshBtn(
            error?.response?.data?.message ??
              error?.message ??
              "Panel duplication failed",
          );
        } else {
          showErrorNotification(error?.message ?? "Panel duplication failed");
        }
      }
      // Hide the loading spinner notification.
      dismiss();
    };

    const deletePanelDialog = async (data: any) => {
      emit("onDeletePanel", props.data.id);
    };

    const movePanelDialog = async (selectedTabId: any) => {
      emit("onMovePanel", props.data.id, selectedTabId);
    };

    const isPanelLoading = ref(false);

    const handleLoadingStateChange = (isLoading: any) => {
      isPanelLoading.value = isLoading;
    };

    const onRefreshPanel = async () => {
      if (isPanelLoading.value) return;

      isPanelLoading.value = true;
      try {
        await emit("refreshPanelRequest", props.data.id);
      } finally {
        isPanelLoading.value = false;
      }
    };
    const createVariableRegex = (name: any) =>
      new RegExp(
        `.*\\$\\{?${name}(?::(csv|pipe|doublequote|singlequote))?}?.*`,
      );

    const getDependentVariablesData = () =>
      props.variablesData?.values
        ?.filter((it: any) => it.type != "dynamic_filters") // ad hoc filters are not considered as dependent filters as they are globally applied
        ?.filter((it: any) => {
          const regexForVariable = createVariableRegex(it.name);
          return props.data.queries
            ?.map((q: any) => regexForVariable.test(q?.query))
            ?.includes(true);
        });

    // Check if any dependent variable's value has changed
    const variablesDataUpdated = computed(() => {
      // Get dependent variables
      const dependentVariables = getDependentVariablesData();

      // Validate dependentVariables and currentVariablesData
      if (!Array.isArray(dependentVariables)) {
        return false;
      }
      if (!Array.isArray(props.currentVariablesData.values)) {
        return false;
      }

      // Check if any dependent variable's value has changed
      return dependentVariables.some((dependentVariable) => {
        if (!dependentVariable || !dependentVariable.name) {
          return false;
        }

        // Find the corresponding variable in currentVariablesData
        const refreshedVariable = props.currentVariablesData.values.find(
          (varData: any) => varData.name === dependentVariable.name,
        );

        if (!refreshedVariable) {
          return false; // Assume no change if no matching variable
        }

        // Compare values
        const currentValue = dependentVariable.value;
        const refreshedValue = refreshedVariable.value;

        // Handle both array and primitive types for value comparison
        return Array.isArray(currentValue)
          ? !isEqual(currentValue, refreshedValue)
          : currentValue !== refreshedValue;
      });
    });

    const errorData = ref("");
    const onError = (error: any) => {
      if (typeof error === "string") {
        errorData.value = error;
      } else {
        errorData.value = error?.message ?? "";
      }
    };

    const isPartialData = ref(false);

    const handleIsPartialDataUpdate = (isPartial: boolean) => {
      isPartialData.value = isPartial;
    };

    // Add cleanup on component unmount
    onBeforeUnmount(() => {
      // Clear any pending timeouts or intervals
      // Reset refs to help with garbage collection
      metaData.value = null;
      errorData.value = "";

      // Clear the PanelSchemaRenderer reference
      if (PanleSchemaRendererRef.value) {
        PanleSchemaRendererRef.value = null;
      }
    });

    return {
      props,
      onEditPanel,
      onLogPanel,
      onDuplicatePanel,
      deletePanelDialog,
      isCurrentlyHoveredPanel,
      outlinedWarning,
      symOutlinedClockLoader20,
      symOutlinedDataInfoAlert,
      outlinedRunningWithErrors,
      store,
      metaDataValue,
      handleResultMetadataUpdate,
      handleLastTriggeredAtUpdate,
      isCachedDataDifferWithCurrentTimeRange,
      handleIsCachedDataDifferWithCurrentTimeRangeUpdate,
      lastTriggeredAt,
      maxQueryRange,
      metaData,
      showViewPanel,
      dependentAdHocVariable,
      confirmDeletePanelDialog,
      showText,
      PanleSchemaRendererRef,
      confirmMovePanelDialog,
      movePanelDialog,
      onRefreshPanel,
      variablesDataUpdated,
      onError,
      errorData,
      isPanelLoading,
      handleLoadingStateChange,
      limitNumberOfSeriesWarningMessage,
      handleLimitNumberOfSeriesWarningMessageUpdate,
      isPartialData,
      handleIsPartialDataUpdate,
    };
  },
  methods: {
    onPanelModifyClick(evt: any) {
      if (evt == "ViewPanel") {
        this.$emit("onViewPanel", this.props.data.id);
      } else if (evt == "EditPanel") {
        this.onEditPanel(this.props.data);
      } else if (evt == "DeletePanel") {
        this.confirmDeletePanelDialog = true;
      } else if (evt == "DuplicatePanel") {
        this.onDuplicatePanel(this.props.data);
      } else if (evt == "MovePanel") {
        this.confirmMovePanelDialog = true;
      } else if (evt == "EditLayout") {
        this.$emit("onEditLayout", this.props.data.id);
      } else {
      }
    },
  },
});
</script>

<style lang="scss" scoped>
.panelcontainer {
  height: calc(100% - 24px);
}

.panelHeader {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.warning {
  color: var(--q-warning);
}

.lastRefreshedAt {
  font-size: smaller;
  margin-left: 5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;

  &::after {
    content: "";
  }

  &::before {
    content: "";
  }

  & .lastRefreshedAtIcon {
    font-size: smaller;
    margin-right: 2px;
  }
}
</style>
