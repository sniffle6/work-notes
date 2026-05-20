<script lang="ts">
  type Tone = "neutral" | "accent" | "hot" | "success" | "warning" | "error";

  type Props = {
    label: string;
    tone?: Tone;
  };

  let { label, tone = "neutral" }: Props = $props();
</script>

<span class={`status-badge tone-${tone}`}>{label}</span>

<style>
  .status-badge {
    display: inline-flex;
    align-items: center;
    min-height: 20px;
    padding: 1px 7px;
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    color: var(--color-text-muted);
    background: color-mix(in srgb, var(--color-surface-2) 82%, transparent);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
    white-space: nowrap;
  }

  .tone-accent {
    border-color: var(--color-accent-primary);
    color: var(--color-text-primary);
  }

  .tone-hot {
    border-color: var(--color-accent-hot);
    color: var(--color-accent-hot);
  }

  .tone-success {
    border-color: var(--color-status-success);
    color: var(--color-status-success);
  }

  .tone-warning {
    border-color: var(--color-status-warning);
    color: var(--color-status-warning);
  }

  .tone-error {
    border-color: var(--color-status-error);
    color: var(--color-status-error);
  }
</style>
