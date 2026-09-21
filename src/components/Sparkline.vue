<script setup lang="ts">
import { computed } from "vue";
import type { UsagePoint } from "../types";
import { formatCredits } from "../lib/format";

const props = withDefaults(
  defineProps<{
    points: UsagePoint[];
    metric?: "credit" | "tokens";
  }>(),
  { metric: "credit" },
);

const values = computed(() => props.points.map((point) => (props.metric === "tokens" ? point.tokens : point.credit)));
const maxValue = computed(() => Math.max(...values.value, 0));
const path = computed(() => {
  if (!values.value.length) return "";
  const width = 352;
  const height = 72;
  const step = values.value.length > 1 ? width / (values.value.length - 1) : width;
  return values.value
    .map((value, index) => {
      const x = index * step;
      const ratio = maxValue.value > 0 ? value / maxValue.value : 0;
      const y = height - 7 - ratio * (height - 18);
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
});
const areaPath = computed(() => {
  if (!path.value) return "";
  const lastX = Math.max(0, values.value.length - 1) * (352 / Math.max(1, values.value.length - 1));
  return `${path.value} L ${lastX.toFixed(2)} 72 L 0 72 Z`;
});
const total = computed(() => values.value.reduce((sum, value) => sum + value, 0));
</script>

<template>
  <div class="sparkline-wrap">
    <div class="sparkline-head">
      <span>近 30 天{{ metric === "credit" ? "扣费" : "Token" }}</span>
      <b>{{ metric === "credit" ? formatCredits(total) : total.toLocaleString("zh-CN") }}</b>
    </div>
    <svg v-if="points.length" class="sparkline" viewBox="0 0 352 72" preserveAspectRatio="none" aria-hidden="true">
      <defs>
        <linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.34" />
          <stop offset="100%" stop-color="#f59e0b" stop-opacity="0" />
        </linearGradient>
      </defs>
      <path :d="areaPath" fill="url(#spark-fill)" />
      <path :d="path" fill="none" stroke="#f59e0b" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    <div v-else class="sparkline-empty">暂无用量数据</div>
  </div>
</template>
