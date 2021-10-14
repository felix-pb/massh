<template>
  <v-menu offset-y>
    <template v-slot:activator="{ on }">
      <v-btn text v-on="on">
        <v-icon>invert_colors</v-icon>
      </v-btn>
    </template>
    <v-list>
      <v-list-item
        :key="`app-color-${i}`"
        v-for="(color, i) in colors"
        @click="toggleAppColor(color)"
      >
        <v-icon class="pl-1" :color="color">circle</v-icon>
      </v-list-item>
    </v-list>
  </v-menu>
</template>

<script lang="ts">
import Vue from "vue";

export default Vue.extend({
  name: "AppColor",
  data() {
    return {
      colors: [
        "#BA68C8",
        "#E91E63",
        "#F44336",
        "#FF9800",
        "#4CAF50",
        "#009688",
        "#2196F3",
      ],
    };
  },
  methods: {
    toggleAppColor(appColor: string): void {
      this.$vuetify.theme.themes.dark.primary = appColor;
      this.$vuetify.theme.themes.light.primary = appColor;
      localStorage.setItem("AppColor", appColor);
    },
  },
  mounted(): void {
    const appColor = localStorage.getItem("AppColor");
    if (appColor) {
      this.$vuetify.theme.themes.dark.primary = appColor;
      this.$vuetify.theme.themes.light.primary = appColor;
    }
  },
});
</script>
