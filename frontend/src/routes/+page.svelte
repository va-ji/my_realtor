<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchProperties } from '$lib/api';
	import type { Property } from '$lib/types';

	let properties: Property[] = $state([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedState = $state<string>('ALL');

	onMount(async () => {
		try {
			properties = await fetchProperties();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load properties';
		} finally {
			loading = false;
		}
	});

	const states = ['ALL', 'VIC', 'NSW', 'QLD', 'WA', 'SA', 'TAS', 'ACT', 'NT'];

	const filteredProperties = $derived(
		selectedState === 'ALL'
			? properties
			: properties.filter((p) => p.state === selectedState)
	);

	function formatPrice(price: number | null): string {
		if (!price) return 'N/A';
		return new Intl.NumberFormat('en-AU', {
			style: 'currency',
			currency: 'AUD',
			minimumFractionDigits: 0
		}).format(price);
	}

	function formatYield(yieldVal: number | null): string {
		if (!yieldVal) return 'N/A';
		return `${yieldVal.toFixed(2)}%`;
	}
</script>

<div class="container">
	<header>
		<h1>🏠 Australian Real Estate Analysis</h1>
		<p>Investment property rental yield calculator and comparison tool</p>
	</header>

	<div class="filters">
		<label for="state-filter">Filter by State:</label>
		<select id="state-filter" bind:value={selectedState}>
			{#each states as state}
				<option value={state}>{state}</option>
			{/each}
		</select>
		<span class="count"
			>{filteredProperties.length} {filteredProperties.length === 1 ? 'property' : 'properties'}</span
		>
	</div>

	{#if loading}
		<div class="loading">Loading properties...</div>
	{:else if error}
		<div class="error">
			<p>❌ {error}</p>
			<p>Make sure the backend API is running on http://localhost:3001</p>
		</div>
	{:else if filteredProperties.length === 0}
		<div class="empty">
			<p>No properties found {selectedState !== 'ALL' ? `in ${selectedState}` : ''}</p>
		</div>
	{:else}
		<div class="properties-grid">
			{#each filteredProperties as property (property.id)}
				<div class="property-card">
					<div class="property-header">
						<h3>{property.address}</h3>
						<span class="state-badge">{property.state}</span>
					</div>
					<div class="property-location">
						<span class="suburb">{property.suburb}</span>
					</div>
					<div class="property-details">
						<div class="detail">
							<span class="label">Bedrooms:</span>
							<span class="value">{property.bedrooms ?? 'N/A'}</span>
						</div>
						<div class="detail">
							<span class="label">Price:</span>
							<span class="value">{formatPrice(property.price)}</span>
						</div>
						<div class="detail">
							<span class="label">Weekly Rent:</span>
							<span class="value">{formatPrice(property.weekly_rent)}</span>
						</div>
						<div class="detail highlight">
							<span class="label">Rental Yield:</span>
							<span class="value yield">{formatYield(property.rental_yield)}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
		font-family: system-ui, -apple-system, sans-serif;
	}

	header {
		text-align: center;
		margin-bottom: 3rem;
	}

	h1 {
		font-size: 2.5rem;
		color: #2c3e50;
		margin-bottom: 0.5rem;
	}

	header p {
		color: #7f8c8d;
		font-size: 1.1rem;
	}

	.filters {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-bottom: 2rem;
		padding: 1rem;
		background: #f8f9fa;
		border-radius: 8px;
	}

	label {
		font-weight: 600;
		color: #2c3e50;
	}

	select {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
		cursor: pointer;
	}

	.count {
		margin-left: auto;
		color: #7f8c8d;
		font-weight: 500;
	}

	.loading,
	.error,
	.empty {
		text-align: center;
		padding: 3rem;
		font-size: 1.2rem;
		color: #7f8c8d;
	}

	.error {
		color: #e74c3c;
	}

	.error p {
		margin: 0.5rem 0;
	}

	.properties-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 1.5rem;
	}

	.property-card {
		background: white;
		border: 1px solid #e0e0e0;
		border-radius: 8px;
		padding: 1.5rem;
		transition: all 0.2s;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
	}

	.property-card:hover {
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
		transform: translateY(-2px);
	}

	.property-header {
		display: flex;
		justify-content: space-between;
		align-items: start;
		gap: 1rem;
		margin-bottom: 0.5rem;
	}

	h3 {
		font-size: 1.1rem;
		color: #2c3e50;
		margin: 0;
		flex: 1;
	}

	.state-badge {
		background: #3498db;
		color: white;
		padding: 0.25rem 0.75rem;
		border-radius: 12px;
		font-size: 0.85rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.property-location {
		margin-bottom: 1rem;
	}

	.suburb {
		color: #7f8c8d;
		font-size: 0.95rem;
	}

	.property-details {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		border-top: 1px solid #ecf0f1;
		padding-top: 1rem;
	}

	.detail {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.detail.highlight {
		background: #ecf9f2;
		padding: 0.75rem;
		border-radius: 6px;
		margin-top: 0.5rem;
	}

	.label {
		color: #7f8c8d;
		font-size: 0.9rem;
		font-weight: 500;
	}

	.value {
		color: #2c3e50;
		font-weight: 600;
		font-size: 1rem;
	}

	.yield {
		color: #27ae60;
		font-size: 1.2rem;
	}
</style>