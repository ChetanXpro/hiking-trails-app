<script lang="ts">
	interface RouteChoice {
		id: string;
		name: string;
		tag: string | null;
		network: string | null;
	}

	interface Props {
		choices?: RouteChoice[];
		position?: { x: number; y: number } | null;
		hoveredId?: string | null;
		onSelect?: (id: string) => void;
		onHover?: (id: string | null) => void;
		onClose?: () => void;
	}

	let {
		choices = [],
		position = null,
		hoveredId = null,
		onSelect,
		onHover,
		onClose
	}: Props = $props();


</script>

{#if choices.length > 1 && position}
	<div class="route-chooser" style="left: {position.x}px; top: {position.y}px;">
		<div class="route-chooser-title">
			<span class="route-chooser-label">{choices.length} routes match your click</span>
			<button type="button" class="close-button" onclick={onClose} aria-label="Dismiss route options">
				×
			</button>
		</div>

		<div class="route-options">
			{#each choices as choice}
				<button
					class:hovered={hoveredId === choice.id}
					class="route-option"
					type="button"
					onmouseenter={() => onHover?.(choice.id)}
					onmouseleave={() => onHover?.(null)}
					onclick={() => onSelect?.(choice.id)}
					aria-label="Select {choice.name}"
				>
					<strong>{choice.name}</strong>
					{#if choice.tag}
						<span class="route-tag">{choice.tag}</span>
					{/if}
					{#if choice.network}
						<span class="route-network">{choice.network}</span>
					{/if}
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.route-chooser {
		position: fixed;
		z-index: 40;
		background: rgba(250, 247, 238, 0.96);
		border: 1px solid rgba(0, 0, 0, 0.1);
		border-radius: 8px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		max-width: 280px;
		pointer-events: auto;
	}

	.route-chooser-title {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid rgba(0, 0, 0, 0.08);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.route-chooser-label {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: rgba(0, 0, 0, 0.5);
	}

	.close-button {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: rgba(0, 0, 0, 0.5);
		padding: 0;
		line-height: 1;
	}

	.close-button:hover {
		color: rgba(0, 0, 0, 0.8);
	}

	.route-options {
		display: flex;
		flex-direction: column;
		padding: 0.5rem;
		gap: 0.25rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.route-option {
		padding: 0.75rem;
		border: 1px solid rgba(0, 0, 0, 0.08);
		border-radius: 6px;
		background: rgba(250, 247, 238, 0.95);
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.route-option:hover,
	.route-option.hovered {
		background: rgba(46, 102, 83, 0.1);
		border-color: rgba(46, 102, 83, 0.3);
		transform: translateX(2px);
	}

	.route-option strong {
		color: rgba(0, 0, 0, 0.8);
		font-weight: 600;
	}

	.route-tag,
	.route-network {
		font-size: 0.75rem;
		color: rgba(0, 0, 0, 0.5);
		font-weight: 500;
	}
</style>
