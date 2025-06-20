<template>
  <div>
    <div class="q-mb-sm" style="font-weight: 600">
      <span>Override Config</span>
      <q-btn
        no-caps
        padding="xs"
        size="sm"
        flat
        icon="info_outline"
        data-test="dashboard-addpanel-config-drilldown-info"
      >
        <q-tooltip
          class="bg-grey-8"
          anchor="bottom middle"
          self="top middle"
          max-width="250px"
        >
          Map units to the selected columns for enhanced data visualization.
        </q-tooltip>
      </q-btn>
    </div>

    <q-btn
      @click="openOverrideConfigPopup"
      style="cursor: pointer; padding: 0px 5px"
      :label="' Add field override'"
      no-caps
      data-test="dashboard-addpanel-config-override-config-add-btn"
    />

    <q-dialog v-model="showOverrideConfigPopup">
      <OverrideConfigPopup
        :columns="columns"
        :override-config="
          JSON.parse(
            JSON.stringify(dashboardPanelData.data.config.override_config),
          )
        "
        @close="showOverrideConfigPopup = false"
        @save="saveOverrideConfigConfig"
        :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, inject, onBeforeMount } from "vue";
import { useStore } from "vuex";
import OverrideConfigPopup from "../OverrideConfigPopup.vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";

interface Column {
  alias: string;
  label: string;
  format?: (val: unknown) => string;
}

export interface OverrideConfig {
  [key: string]: string;
}

export default defineComponent({
  name: "OverrideConfig",
  components: { OverrideConfigPopup },
  setup() {
    const store = useStore();
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );
    const { dashboardPanelData } = useDashboardPanelData(
      dashboardPanelDataPageKey,
    );

    const showOverrideConfigPopup = ref(false);
    const columns: any = ref<Column[]>([]);
    const overrideConfigs = ref<OverrideConfig[]>([]);

    const fetchColumns = () => {
      const x = dashboardPanelData.data.queries[0].fields.x || [];
      const y = dashboardPanelData.data.queries[0].fields.y || [];
      columns.value = [...x, ...y];
    };

    const openOverrideConfigPopup = () => {
      fetchColumns();
      showOverrideConfigPopup.value = true;
    };

    const saveOverrideConfigConfig = (overrideConfig: any) => {
      dashboardPanelData.data.config.override_config = overrideConfig;
      applyOverrideConfigs();
      showOverrideConfigPopup.value = false;
    };

    const applyOverrideConfigs = () => {
      const overrides = dashboardPanelData.data.config.override_config || [];

      columns.value = columns.value.map((col: any) => {
        return {
          name: col.alias,
          label: col.label,
          field: col.alias,
          format: (val: any) => {
            const unit = overrides[col.alias] || "";
            return `${val} ${unit}`;
          },
        };
      });
    };

    fetchColumns();
    applyOverrideConfigs();

    onBeforeMount(() => {
      // Ensure that the override_config object is initialized in config
      if (!dashboardPanelData.data.config.override_config) {
        dashboardPanelData.data.config.override_config = [];
      }
    });

    return {
      store,
      dashboardPanelData,
      showOverrideConfigPopup,
      openOverrideConfigPopup,
      saveOverrideConfigConfig,
      columns,
      overrideConfigs,
    };
  },
});
</script>
