            water += raw.xyz * cell.weights[k];
        }
        row.water = water;
    }
    return chart_row(point, body, row, rows);
}
