        var chart_water = false;
        if kind == 2u {
            if chart_contains(cell, body) {
                chart_water = cell.bed < row.water.x;
            } else {
                row = continued_chart(cell, body, row);
                if cell.bed < row.water.x { continue; }
            }
        }
