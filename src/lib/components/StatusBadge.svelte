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
  /* A coloured dot and plain text: state reads at a glance without a box. */
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.4;
    white-space: nowrap;
  }

  .status-badge::before {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    content: "";
  }

  .tone-accent {
    color: var(--color-text-primary);
  }

  .tone-hot {
    color: var(--color-accent-hot);
  }

  .tone-success {
    color: var(--color-status-success);
  }

  .tone-warning {
    color: var(--color-status-warning);
  }

  .tone-error {
    color: var(--color-status-error);
  }
</style>
