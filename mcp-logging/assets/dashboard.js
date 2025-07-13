// Dashboard JavaScript functionality

// Global chart instances
const charts = new Map();

// Chart.js default configuration
Chart.defaults.responsive = true;
Chart.defaults.maintainAspectRatio = false;
Chart.defaults.plugins.legend.display = true;
Chart.defaults.plugins.tooltip.enabled = true;

// Initialize a chart
function initChart(chartId, config, data) {
    const canvas = document.getElementById(`chart-${chartId}`);
    if (!canvas) {
        console.error(`Canvas element for chart ${chartId} not found`);
        return;
    }

    const ctx = canvas.getContext('2d');
    
    // Destroy existing chart if it exists
    if (charts.has(chartId)) {
        charts.get(chartId).destroy();
    }

    try {
        const chartConfig = createChartConfig(config, data);
        const chart = new Chart(ctx, chartConfig);
        charts.set(chartId, chart);
        
        console.log(`Chart ${chartId} initialized successfully`);
    } catch (error) {
        console.error(`Failed to initialize chart ${chartId}:`, error);
        showChartError(canvas, 'Failed to initialize chart');
    }
}

// Create Chart.js configuration from dashboard config
function createChartConfig(config, data) {
    const chartType = getChartJsType(config.chart_type);
    const datasets = createDatasets(config, data);
    
    return {
        type: chartType,
        data: {
            datasets: datasets
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: {
                duration: config.options.animated ? 750 : 0
            },
            scales: createScales(config),
            plugins: {
                legend: {
                    display: config.styling.show_legend,
                    labels: {
                        color: config.styling.text_color,
                        font: {
                            family: config.styling.font_family,
                            size: config.styling.font_size
                        }
                    }
                },
                tooltip: {
                    enabled: true,
                    mode: 'index',
                    intersect: false,
                    callbacks: {
                        label: function(context) {
                            const label = context.dataset.label || '';
                            const value = formatValue(context.parsed.y, config);
                            return `${label}: ${value}`;
                        }
                    }
                }
            },
            interaction: {
                mode: 'nearest',
                axis: 'x',
                intersect: false
            },
            elements: {
                point: {
                    radius: 3,
                    hoverRadius: 6
                },
                line: {
                    tension: 0.1
                }
            }
        }
    };
}

// Convert dashboard chart type to Chart.js type
function getChartJsType(dashboardType) {
    const typeMap = {
        'line_chart': 'line',
        'area_chart': 'line',
        'bar_chart': 'bar',
        'pie_chart': 'pie',
        'gauge_chart': 'doughnut',
        'scatter_plot': 'scatter',
        'sparkline': 'line'
    };
    
    return typeMap[dashboardType] || 'line';
}

// Create datasets from configuration and data
function createDatasets(config, data) {
    const datasets = [];
    
    for (const series of data.series) {
        const dataSource = config.data_sources.find(ds => ds.id === series.id);
        if (!dataSource) continue;
        
        const dataset = {
            label: series.name,
            data: series.data.map(point => ({
                x: new Date(point.timestamp),
                y: point.value
            })),
            borderColor: series.color,
            backgroundColor: config.chart_type === 'area_chart' 
                ? addAlpha(series.color, 0.2) 
                : series.color,
            fill: config.chart_type === 'area_chart',
            borderWidth: 2,
            pointBackgroundColor: series.color,
            pointBorderColor: series.color,
            tension: 0.1
        };
        
        // Apply line style
        if (series.line_style === 'dashed') {
            dataset.borderDash = [5, 5];
        } else if (series.line_style === 'dotted') {
            dataset.borderDash = [2, 2];
        } else if (series.line_style === 'dash_dot') {
            dataset.borderDash = [10, 5, 2, 5];
        }
        
        datasets.push(dataset);
    }
    
    return datasets;
}

// Create scales configuration
function createScales(config) {
    const scales = {};
    
    if (config.styling.show_axes) {
        scales.x = {
            type: 'time',
            display: true,
            title: {
                display: !!config.options.x_label,
                text: config.options.x_label || '',
                color: config.styling.text_color,
                font: {
                    family: config.styling.font_family,
                    size: config.styling.font_size
                }
            },
            grid: {
                display: config.styling.show_grid,
                color: config.styling.grid_color
            },
            ticks: {
                color: config.styling.text_color,
                font: {
                    family: config.styling.font_family,
                    size: config.styling.font_size
                }
            }
        };
        
        scales.y = {
            display: true,
            title: {
                display: !!config.options.y_label,
                text: config.options.y_label || '',
                color: config.styling.text_color,
                font: {
                    family: config.styling.font_family,
                    size: config.styling.font_size
                }
            },
            grid: {
                display: config.styling.show_grid,
                color: config.styling.grid_color
            },
            ticks: {
                color: config.styling.text_color,
                font: {
                    family: config.styling.font_family,
                    size: config.styling.font_size
                },
                callback: function(value) {
                    return formatValue(value, config);
                }
            }
        };
        
        // Apply Y-axis limits
        if (config.options.y_min !== null) {
            scales.y.min = config.options.y_min;
        }
        if (config.options.y_max !== null) {
            scales.y.max = config.options.y_max;
        }
    }
    
    return scales;
}

// Format values for display
function formatValue(value, config) {
    if (typeof value !== 'number') return value;
    
    // Determine format based on metric type or value range
    if (value < 1 && value > 0) {
        return value.toFixed(3);
    } else if (value < 100) {
        return value.toFixed(2);
    } else if (value < 1000) {
        return value.toFixed(1);
    } else if (value < 1000000) {
        return (value / 1000).toFixed(1) + 'K';
    } else {
        return (value / 1000000).toFixed(1) + 'M';
    }
}

// Add alpha channel to color
function addAlpha(color, alpha) {
    if (color.startsWith('#')) {
        const hex = color.slice(1);
        const r = parseInt(hex.slice(0, 2), 16);
        const g = parseInt(hex.slice(2, 4), 16);
        const b = parseInt(hex.slice(4, 6), 16);
        return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
    return color;
}

// Show error message in chart container
function showChartError(canvas, message) {
    const container = canvas.parentElement;
    container.innerHTML = `<div class="chart-error">${message}</div>`;
}

// Show loading message in chart container
function showChartLoading(chartId) {
    const canvas = document.getElementById(`chart-${chartId}`);
    if (canvas) {
        const container = canvas.parentElement;
        container.innerHTML = `<div class="chart-loading">Loading chart data...</div>`;
    }
}

// Refresh dashboard data
async function refreshDashboard() {
    const refreshBtn = document.getElementById('refresh-btn');
    const lastUpdated = document.getElementById('last-updated');
    
    if (refreshBtn) {
        refreshBtn.disabled = true;
        refreshBtn.textContent = '🔄 Refreshing...';
    }
    
    try {
        // Fetch fresh data from the server
        const response = await fetch('/api/dashboard/data');
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const dashboardData = await response.json();
        
        // Update each chart with fresh data
        for (const [chartId, chart] of charts) {
            const chartData = dashboardData.charts[chartId];
            if (chartData) {
                updateChart(chartId, chartData);
            }
        }
        
        // Update last updated timestamp
        if (lastUpdated) {
            lastUpdated.textContent = `Last updated: ${new Date().toLocaleString()}`;
        }
        
        console.log('Dashboard refreshed successfully');
        
    } catch (error) {
        console.error('Failed to refresh dashboard:', error);
        
        // Show error message (you could implement a notification system)
        if (lastUpdated) {
            lastUpdated.textContent = `Refresh failed: ${error.message}`;
            lastUpdated.style.color = '#dc3545';
        }
    } finally {
        if (refreshBtn) {
            refreshBtn.disabled = false;
            refreshBtn.textContent = '🔄 Refresh';
        }
    }
}

// Update existing chart with new data
function updateChart(chartId, newData) {
    const chart = charts.get(chartId);
    if (!chart) {
        console.warn(`Chart ${chartId} not found for update`);
        return;
    }
    
    try {
        // Update datasets
        for (let i = 0; i < chart.data.datasets.length; i++) {
            const dataset = chart.data.datasets[i];
            const series = newData.series.find(s => s.name === dataset.label);
            
            if (series) {
                dataset.data = series.data.map(point => ({
                    x: new Date(point.timestamp),
                    y: point.value
                }));
            }
        }
        
        // Update the chart
        chart.update('none'); // No animation for updates
        
    } catch (error) {
        console.error(`Failed to update chart ${chartId}:`, error);
    }
}

// Auto-refresh functionality
let autoRefreshInterval;

function startAutoRefresh(intervalSeconds) {
    stopAutoRefresh(); // Clear any existing interval
    
    if (intervalSeconds > 0) {
        autoRefreshInterval = setInterval(refreshDashboard, intervalSeconds * 1000);
        console.log(`Auto-refresh started with ${intervalSeconds}s interval`);
    }
}

function stopAutoRefresh() {
    if (autoRefreshInterval) {
        clearInterval(autoRefreshInterval);
        autoRefreshInterval = null;
        console.log('Auto-refresh stopped');
    }
}

// Initialize dashboard when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
    console.log('Dashboard loaded');
    
    // Start auto-refresh if configured
    const refreshInterval = window.dashboardConfig?.refresh_interval_secs || 30;
    if (refreshInterval > 0) {
        startAutoRefresh(refreshInterval);
    }
    
    // Add keyboard shortcuts
    document.addEventListener('keydown', function(event) {
        if (event.ctrlKey || event.metaKey) {
            switch (event.key) {
                case 'r':
                    event.preventDefault();
                    refreshDashboard();
                    break;
            }
        }
    });
});

// Clean up on page unload
window.addEventListener('beforeunload', function() {
    stopAutoRefresh();
    
    // Destroy all charts
    for (const chart of charts.values()) {
        chart.destroy();
    }
    charts.clear();
});