<script setup>
import { ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import Column from "primevue/column";
import Dialog from "primevue/dialog";
import DataTable from "primevue/datatable";

// v-model:visible
const visible = defineModel("visible", { type: Boolean });

// propsで項目受け取り
const props = defineProps({
  headers: {
    type: Object,
    default: () => ({
      col1: "項目名1",
      col2: "項目名2",
      col3: "ファイルパス",
    }),
  },
  initialItems: {
    type: Array,
    required: true,
  },
});

// emit
const emit = defineEmits(["ok"]);

// 内部のitems（path付き）
const items = ref([]);

// visibleがtrueになったときにpropsからコピー
watch(visible, (val) => {
  if (val) {
    items.value = props.initialItems.map((i) => ({ ...i, path: "" }));
  }
});

async function openFileDialog(item) {
  const selected = await open({
    multiple: false,
    filters: [{ name: "All Files", extensions: ["*"] }],
  });
  if (!selected) {
    return;
  }
  item.path = selected;
  items.value.forEach((i) => {
    if (!i.path) i.path = selected;
  });
}

function onOk() {
  emit("ok", items.value);
  visible.value = false;
}

function onCancel() {
  visible.value = false;
}

const dialogWidth = ref(600);
const minWidth = 400;
let startX = 0;
let startWidth = 0;

function startResize(e) {
  startX = e.clientX;
  startWidth = dialogWidth.value;
  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", stopResize);
  e.preventDefault();
}

function onMouseMove(e) {
  const delta = e.clientX - startX;
  let newWidth = startWidth + delta;
  if (newWidth < minWidth) newWidth = minWidth;
  dialogWidth.value = newWidth;
}

function stopResize() {
  window.removeEventListener("mousemove", onMouseMove);
  window.removeEventListener("mouseup", stopResize);
}
</script>

<template>
  <Dialog
    v-model:visible="visible"
    header="ファイル設定"
    modal
    :style="{ width: dialogWidth + 'px' }"
  >
    <DataTable :value="items" responsiveLayout="scroll">
      <Column outlined field="name" :header="props.headers.col1" />
      <Column outlined field="sub" :header="props.headers.col2" />
      <Column outlined header="ファイルパス">
        <template #body="slotProps">
          <div class="flex items-center gap-2">
            <InputText outlined v-model="slotProps.data.path" class="flex-1" />
            <Button
              icon="pi pi-folder-open"
              outlined
              @click="openFileDialog(slotProps.data)"
            />
          </div>
        </template>
      </Column>
    </DataTable>

    <template #footer>
      <Button outlined label="OK" icon="pi pi-check" @click="onOk" />
      <Button
        outlined
        label="キャンセル"
        icon="pi pi-times"
        severity="secondary"
        @click="onCancel"
      />
    </template>
    <div class="resize-handle" @mousedown="startResize"></div>
  </Dialog>
</template>

<style scoped>
.resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  width: 6px;
  height: 100%;
  cursor: ew-resize;
  z-index: 1000;
}
</style>
