function addGraph(node, data, for_single_file)
{
	var ctx = node.getContext('2d');
	var customTooltip = function(tooltip) {
		// Tooltip Element
		var tooltipEl = document.getElementById('chartjs-tooltip');

		if (!tooltipEl)
		{
			tooltipEl = document.createElement('div');
			tooltipEl.id = 'chartjs-tooltip';
			tooltipEl.innerHTML = '<table></table>';
			document.body.appendChild(tooltipEl);
		}

		// Hide if no tooltip
		if (tooltip.opacity === 0)
		{
			tooltipEl.style.opacity = 0;
			return;
		}

		// Set caret Position
		tooltipEl.classList.remove('above', 'below', 'no-transform');
		if (tooltip.yAlign)
		{
			tooltipEl.classList.add(tooltip.yAlign);
		}
		else
		{
			tooltipEl.classList.add('no-transform');
		}

		function getBody(bodyItem)
		{
			var dataset = data.datasets[bodyItem.datasetIndex];
			var itemData = dataset.data[bodyItem.index];
			if (bodyItem.datasetIndex != 0)
				return dataset.label + ': ' + Math.round(itemData['v'] * 100) / 100 + ' s';
			else
			{
				return dataset.label + ': ' + Math.round(itemData['v']) + ' MB';
			}
		}

		// Set Text
		if (tooltip.body)
		{
			var titleLines = tooltip.title || [];
			var bodyLines = tooltip.dataPoints.map(getBody);

			var innerHtml = '<thead>';

			titleLines.forEach(function(title) { innerHtml += '<tr><th>' + title + '</th></tr>'; });
			innerHtml += '</thead><tbody>';

			bodyLines.forEach(function(body, i) {
				var colors = tooltip.labelColors[i];
				var style = 'background:' + colors.backgroundColor;
				style += '; border-color:' + colors.borderColor;
				style += '; border-width: 2px';
				var span = '<span class="chartjs-tooltip-key" style="' + style + '"></span>';
				innerHtml += '<tr><td>' + span + body + '</td></tr>';
			});
			innerHtml += '</tbody>';

			var tableRoot = tooltipEl.querySelector('table');
			tableRoot.innerHTML = innerHtml;
		}

		var rect = this._chart.canvas.getBoundingClientRect();
		var positionY = rect.top + window.pageYOffset;
		var positionX = rect.left;

		// Display, position, and set styles for font
		tooltipEl.style.opacity = 1;
		tooltipEl.style.left = positionX + tooltip.caretX + 'px';
		tooltipEl.style.top = positionY + tooltip.caretY + 'px';
		tooltipEl.style.padding = tooltip.yPadding + 'px ' + tooltip.xPadding + 'px';
	};
	var options;
	if (for_single_file)
	{
		options = {
			animation: {duration: 0},
			hover: {animationDuration: 0},
			responsiveAnimationDuration: 0,
			elements: {line: {tension: 0}},
			scales: {yAxes: [{ticks: {beginAtZero: true}}]},
			tooltips: {enabled: false, mode: 'index', intersect: false, custom: customTooltip}
		}
	}
	else
	{
		options = {
			animation: {duration: 0},
			hover: {animationDuration: 0},
			legend: {
				display: false,
			},
			responsiveAnimationDuration: 0,
			elements: {line: {tension: 0}},
			scales: {yAxes: [{ticks: {beginAtZero: true}}]}
		}
	}
	var chart = new Chart(ctx, {type: 'line', data: data, options: options});
}
function loadChart(chartNode)
{
	var xhttp = new XMLHttpRequest();
	xhttp.onreadystatechange = function() {
		if (this.readyState == 4 && this.status == 200)
		{
			var data = JSON.parse(this.responseText)
			var node = document.createElement('canvas');
			node.classList = ['chartContainer'];
			node.width = 500;
			node.height = 150;
			chartNode.appendChild(node);
			addGraph(node, data, true);
		}
	};
	var chartId = encodeURI(chartNode.getAttribute('data-chart-id'));
	if (chartId.includes('.csb'))
	{
		xhttp.open('GET', '/api/file/csb?id=%' + chartId, true);
	}
	else
	{
		xhttp.open('GET', '/api/file/ini?id=%' + chartId, true);
	}
	xhttp.send();
}
function loadSummaryChart(type, r1, r2)
{
	var xhttp = new XMLHttpRequest();
	xhttp.onreadystatechange = function() {
		if (this.readyState == 4 && this.status == 200)
		{
			var data = JSON.parse(this.responseText)
			var node = document.getElementById(type + '_graph');
			addGraph(node, data, false);
		}
	};
	xhttp.open('GET', '/api/all/' + type + '?r1=' + r1 + '&r2=' + r2, true);
	xhttp.send();
}
function loadSummaryCharts(r1, r2)
{
	document.getElementById('summary_charts').innerHTML = '<h1>Graphs</h1>\
            <h2>CSB Play Time</h2>\
            <canvas id="csb_play_time_graph" width="500" height="100"></canvas>\
            <h2>CSB Memory</h2>\
            <canvas id="csb_memory_graph" width="500" height="100"></canvas>\
            <h2>Ini Cutting Time</h2>\
            <canvas id="ini_cut_time_graph" width="500" height="100"></canvas>\
            <h2>Ini Draw Time</h2>\
            <canvas id="ini_draw_time_graph" width="500" height="100"></canvas>\
            <h2>Ini Memory</h2>\
            <canvas id="ini_memory_graph" width="500" height="100"></canvas>';
	loadSummaryChart('csb_play_time', r1, r2);
	loadSummaryChart('csb_memory', r1, r2);
	loadSummaryChart('ini_cut_time', r1, r2);
	loadSummaryChart('ini_draw_time', r1, r2);
	loadSummaryChart('ini_memory', r1, r2);
}
window.onload = function() {
	for (let element of document.querySelectorAll('.toggle-table'))
	{
		let name = element.parentElement.getAttribute('data-js-name');
		let in_body = [];
		let next = element.parentElement.parentElement.nextElementSibling;
		while (next && next.getAttribute('data-field-start') !== 'true')
		{
			in_body.push(next);
			next = next.nextElementSibling;
		}
		for (let detail of in_body)
		{
			detail.style.display = 'none';
		}
		element.addEventListener('toggle', evt => {
			for (let detail of in_body)
			{
				if (element.open)
				{
					detail.style.display = '';
				}
				else
				{
					detail.style.display = 'none';
				}
				var charts = detail.getElementsByClassName('chart');
				if (charts.length > 0)
				{
					var chart = charts[0];
					if (element.open)
					{
						loadChart(chart);
					}
					else
					{
						detail.getElementsByClassName('chartContainer')[0].remove();
					}
				}
			}
		});
	}
}