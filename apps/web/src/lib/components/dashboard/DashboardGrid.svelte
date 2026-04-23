<script lang="ts">
	import WidgetFactory from './WidgetFactory.svelte';
	import {
		type DashboardFilterState,
		type DashboardWidget,
		type DashboardWidgetLayout,
	} from '$lib/utils/dashboards';

	interface Props {
		widgets: DashboardWidget[];
		filters: DashboardFilterState;
		editing?: boolean;
		onReorder?: (fromIndex: number, toIndex: number) => void;
		onEditWidget?: (widget: DashboardWidget) => void;
		onDeleteWidget?: (widgetId: string) => void;
		onDuplicateWidget?: (widgetId: string) => void;
		onResizeWidget?: (widgetId: string, layout: DashboardWidgetLayout) => void;
	}

	let {
		widgets,
		filters,
		editing = false,
		onReorder,
		onEditWidget,
		onDeleteWidget,
		onDuplicateWidget,
		onResizeWidget,
	}: Props = $props();

	let dragIndex = $state<number | null>(null);
	let overIndex = $state<number | null>(null);

	function handleDragStart(index: number) {
		dragIndex = index;
		overIndex = index;
	}

	function handleDrop(event: DragEvent, targetIndex: number) {
		event.preventDefault();
		if (dragIndex === null || dragIndex === targetIndex) {
			dragIndex = null;
			overIndex = null;
			return;
		}

		onReorder?.(dragIndex, targetIndex);
		dragIndex = null;
		overIndex = null;
	}

	function updateLayout(widget: DashboardWidget, nextLayout: DashboardWidgetLayout) {
		onResizeWidget?.(widget.id, nextLayout);
	}
</script>

{#if widgets.length === 0}
	<div class="flex min-h-[280px] items-center justify-center rounded-3xl border border-dashed border-slate-300 bg-slate-50 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-400">
		Add your first widget to start composing a dashboard.
	</div>
{:else}
	<div class="dashboard-grid">
		{#each widgets as widget, index (widget.id)}
			<div
				class={`dashboard-grid__item ${editing ? 'dashboard-grid__item--editing' : ''} ${overIndex === index ? 'dashboard-grid__item--drop' : ''}`}
				style={`--col-span:${Math.min(Math.max(widget.layout.colSpan, 1), 12)};--col-span-md:${Math.min(Math.max(widget.layout.colSpan, 1), 6)};--col-span-sm:1;--row-span:${Math.min(Math.max(widget.layout.rowSpan, 1), 4)};`}
				role="group"
				draggable={editing}
				ondragstart={() => handleDragStart(index)}
				ondragover={(event) => {
					if (editing) {
						event.preventDefault();
						overIndex = index;
					}
				}}
				ondragleave={() => {
					if (overIndex === index) {
						overIndex = null;
					}
				}}
				ondragend={() => {
					dragIndex = null;
					overIndex = null;
				}}
				ondrop={(event) => handleDrop(event, index)}
			>
				{#if editing}
					<div class="dashboard-grid__toolbar">
						<span class="dashboard-grid__drag">Drag</span>
						<div class="dashboard-grid__toolbar-actions">
							<button title="Edit widget" onclick={() => onEditWidget?.(widget)}>Edit</button>
							<button title="Duplicate widget" onclick={() => onDuplicateWidget?.(widget.id)}>Copy</button>
							<button title="Delete widget" onclick={() => onDeleteWidget?.(widget.id)}>Delete</button>
						</div>
					</div>

					<div class="dashboard-grid__resize">
						<button title="Narrower" onclick={() => updateLayout(widget, { ...widget.layout, colSpan: Math.max(1, widget.layout.colSpan - 1) })}>W-</button>
						<button title="Wider" onclick={() => updateLayout(widget, { ...widget.layout, colSpan: Math.min(12, widget.layout.colSpan + 1) })}>W+</button>
						<button title="Shorter" onclick={() => updateLayout(widget, { ...widget.layout, rowSpan: Math.max(1, widget.layout.rowSpan - 1) })}>H-</button>
						<button title="Taller" onclick={() => updateLayout(widget, { ...widget.layout, rowSpan: Math.min(4, widget.layout.rowSpan + 1) })}>H+</button>
					</div>
				{/if}

				<WidgetFactory widget={widget} filters={filters} />
			</div>
		{/each}
	</div>
{/if}

<style>
	.dashboard-grid {
		display: grid;
		gap: 1rem;
		grid-template-columns: repeat(12, minmax(0, 1fr));
		grid-auto-rows: minmax(180px, auto);
	}

	.dashboard-grid__item {
		position: relative;
		min-width: 0;
		grid-column: span var(--col-span) / span var(--col-span);
		grid-row: span var(--row-span) / span var(--row-span);
	}

	.dashboard-grid__item--editing {
		cursor: grab;
	}

	.dashboard-grid__item--drop::after {
		position: absolute;
		inset: 0;
		border: 2px dashed rgba(15, 118, 110, 0.7);
		border-radius: 1.5rem;
		content: '';
		pointer-events: none;
	}

	.dashboard-grid__toolbar,
	.dashboard-grid__resize {
		position: absolute;
		z-index: 10;
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}

	.dashboard-grid__toolbar {
		top: 0.85rem;
		right: 0.85rem;
	}

	.dashboard-grid__resize {
		bottom: 0.85rem;
		right: 0.85rem;
	}

	.dashboard-grid__drag,
	.dashboard-grid__toolbar button,
	.dashboard-grid__resize button {
		border: 1px solid rgba(148, 163, 184, 0.35);
		background: rgba(15, 23, 42, 0.82);
		color: white;
		border-radius: 999px;
		padding: 0.3rem 0.65rem;
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		backdrop-filter: blur(10px);
	}

	.dashboard-grid__toolbar-actions {
		display: flex;
		gap: 0.4rem;
	}

	@media (max-width: 1024px) {
		.dashboard-grid {
			grid-template-columns: repeat(6, minmax(0, 1fr));
		}

		.dashboard-grid__item {
			grid-column: span var(--col-span-md) / span var(--col-span-md);
		}
	}

	@media (max-width: 720px) {
		.dashboard-grid {
			grid-template-columns: minmax(0, 1fr);
		}

		.dashboard-grid__item {
			grid-column: span var(--col-span-sm) / span var(--col-span-sm);
		}

		.dashboard-grid__toolbar,
		.dashboard-grid__resize {
			position: static;
			margin-bottom: 0.5rem;
			flex-wrap: wrap;
		}
	}
</style>
