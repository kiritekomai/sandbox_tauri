<script setup>
import { ref, reactive, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Dialog from "primevue/dialog";

/* props / emit */
const props = defineProps({ visible: Boolean });
const emit = defineEmits(["update:visible"]);

/* reactive nodes holder */
const nodes = ref([]);

/* 配列オプション（Rust 側から渡す場合はここを置き換えてください） */
const arrayOptions = [
  { label: "arrayA", value: "arrayA" },
  { label: "arrayB", value: "arrayB" },
  { label: "arrayC", value: "arrayC" },
];
const allArrayValues = arrayOptions.map((o) => o.value);

/* raw -> reactive ノード変換。registed の全キーを埋めておくのが重要 */
function makeReactiveNode(raw) {
  const reg = {};
  for (const v of allArrayValues) {
    reg[v] = !!(raw.registed && raw.registed[v]);
  }

  const node = reactive({
    id: raw.id,
    label: raw.label,
    full_path: raw.full_path,
    exists: !!raw.exists,
    selected: !!raw.selected,
    inside_repo: !!raw.inside_repo,
    registed: reg,
    children: [],
  });

  if (Array.isArray(raw.children)) {
    node.children = raw.children.map((c) => makeReactiveNode(c));
  }
  return node;
}

/* ダイアログが開かれたらバックエンドから読み込み、reactive化 */
watch(
  () => props.visible,
  async (val) => {
    if (val) {
      const raw = await invoke("get_include_tree_nodes");
      console.log(raw);
      const arr = Array.isArray(raw) ? raw : [];
      nodes.value = arr.map((n) => makeReactiveNode(n));
    }
  }
);

/* 判定関数 */
function isAllRegistered(node) {
  const vals = allArrayValues.map((k) => !!node.registed[k]);
  return vals.length > 0 && vals.every((v) => v === true);
}
function isPartiallyRegistered(node) {
  const vals = allArrayValues.map((k) => !!node.registed[k]);
  return vals.some((v) => v === true) && !vals.every((v) => v === true);
}

/* チェックボックスの全ON/全OFF */
function toggleAll(node, newModelValue) {
  // newModelValue は boolean（binary モード）
  for (const k of allArrayValues) node.registed[k] = !!newModelValue;
  // デバッグ用
  // console.log("toggleAll:", node.id, node.registed);
}

/* MultiSelect 側との同期関数 */
function registedToArray(node) {
  return allArrayValues.filter((k) => node.registed[k]);
}
function arrayToRegisted(node, selectedArray) {
  // selectedArray: string[] (may be [])
  for (const k of allArrayValues) {
    node.registed[k] =
      Array.isArray(selectedArray) && selectedArray.includes(k);
  }
  // デバッグ用
  // console.log("arrayToRegisted:", node.id, node.registed);
}
</script>

<template>
  <Dialog
    :visible="props.visible"
    modal
    header="フォルダ選択"
    style="width: 500vw"
    @update:visible="emit('update:visible', $event)"
  >
    <Tree :value="nodes">
      <template #default="slotProps">
        <div>
          <i
            v-if="slotProps.node.exists"
            class="pi pi-folder text-yellow-500"
          />
          <i v-else class="pi pi-exclamation-triangle text-red-500" />

          <span>{{ slotProps.node.label }}</span>

          <!-- ← ここが重要: binary を付けて boolean モードにする -->
          <Checkbox
            binary
            :modelValue="isAllRegistered(slotProps.node)"
            :indeterminate="isPartiallyRegistered(slotProps.node)"
            @update:modelValue="(val) => toggleAll(slotProps.node, val)"
          />

          <MultiSelect
            :modelValue="registedToArray(slotProps.node)"
            :options="arrayOptions"
            optionLabel="label"
            optionValue="value"
            display="chip"
            placeholder="配列を選択"
            class="w-48"
            @update:modelValue="(val) => arrayToRegisted(slotProps.node, val)"
          />
        </div>
      </template>
    </Tree>
  </Dialog>
</template>

<style>
.pi-folder {
  color: #facc15;
}
.pi-exclamation-triangle {
  color: #ef4444;
}
</style>
