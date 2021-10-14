<template>
  <v-btn text @click="toggleAppTheme()">
    <v-icon>settings_brightness</v-icon>
  </v-btn>
</template>

<script lang="ts">
import Vue from "vue";

export default Vue.extend({
  name: "AppTheme",
  methods: {
    toggleAppTheme(): void {
      const isDark = !this.$vuetify.theme.dark;
      this.$vuetify.theme.dark = isDark;
      document.body.style.backgroundColor = isDark ? "#121212" : "#FFFFFF";
      const appTheme = isDark ? "dark" : "light";
      localStorage.setItem("AppTheme", appTheme);
    },
  },
  mounted(): void {
    const appTheme = localStorage.getItem("AppTheme");
    if (appTheme) {
      const isDark = appTheme === "dark";
      this.$vuetify.theme.dark = isDark;
      document.body.style.backgroundColor = isDark ? "#121212" : "#FFFFFF";
    }
  },
});
</script>

<style>
body {
  background-color: #121212;
}
</style>
