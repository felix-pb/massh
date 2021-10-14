<template>
  <v-app>
    <v-app-bar app>
      <v-btn color="primary" outlined to="/">massh</v-btn>
      <v-spacer></v-spacer>
      <app-color></app-color>
      <app-theme></app-theme>
    </v-app-bar>
    <v-main class="ma-5">
      <router-view></router-view>
    </v-main>
  </v-app>
</template>

<script lang="ts">
import Vue from "vue";
import AppColor from "./components/AppColor.vue";
import AppTheme from "./components/AppTheme.vue";

export default Vue.extend({
  name: "App",
  components: {
    AppColor,
    AppTheme,
  },
});
</script>
