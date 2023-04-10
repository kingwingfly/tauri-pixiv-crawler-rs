<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/tauri";


const state = ref("");
const process = ref("");
const save_path = ref("");
const uuid = ref("");
const cookie = ref("");
const path = ref("");
const proxy = ref("");
const working = ref(false);


async function go() {
  if (uuid.value == '' || cookie.value == '') {
    state.value = "Missing param"
    return
  };
  state.value = "Downloading";
  invoke("go", { uuid: uuid.value, cookie: cookie.value, path: path.value, proxy: proxy.value });
  working.value = true;
  save_path.value = await invoke("save_path");
  while (working.value) {
    process.value = await invoke("process");
    let temp = process.value.split("/");
    if (temp[0] == temp[1] && temp[1] != "0") {
      state.value = "Finished";
      invoke("interrupt");
      working.value = false;
    }
    await new Promise(f => setTimeout(f, 1000));
  }
  process.value = "";
}

async function interrupt() {
  if (working.value) {
    invoke("interrupt")
    state.value = "interrupted";
    working.value = false;
  } else {
    state.value = "haven't work yet";
  }
}


async function init() {
  path.value = await invoke("download_dir");
  let hm: { [key: string]: string } = await invoke("get_cached_config");
  uuid.value = hm["uuid"];
  cookie.value = hm["cookie"];
  path.value = hm["path"];
  proxy.value = hm["proxy"];
}

init()

</script>

<template>
  <div class="card">
    <label>Artist ID:</label>
    <input id="id-input" v-model="uuid" placeholder="Enter a pixiv artist id..." />
    <br>
    <label>Cookie:</label>
    <input id="cookie-input" v-model="cookie" placeholder="Enter your cookie" />
    <br>
    <label>Save path (Optional):</label>
    <input id="path-input" v-model="path" placeholder="Enter your path" />
    <br>
    <label>Proxy (Optional):</label>
    <input id="proxy-input" v-model="proxy" placeholder="Enter your proxy" />
    <br>
    <div>
      <button type="button" @click="go()">Go</button>
      <button type="button" @click="interrupt()">Interrupt</button>
    </div>
    <br>
    <div>{{ state }}</div>
    <div>{{ process }}</div>
    <div v-if="save_path">Save path: {{ save_path }}</div>
  </div>
</template>

<style>
.card {
  display: flex;
  flex-direction: column;
  align-items: center;
}
</style>