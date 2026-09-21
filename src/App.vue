<script setup lang="ts">
import LoginView from "./components/LoginView.vue";
import PopoverView from "./components/PopoverView.vue";
import SettingsView from "./components/SettingsView.vue";

const view = new URLSearchParams(window.location.search).get("view") ?? "login";
</script>

<template>
  <LoginView v-if="view === 'login'" />
  <SettingsView v-else-if="view === 'settings'" />
  <PopoverView v-else />
</template>
