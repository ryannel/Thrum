            let tolerance = select(0.000002, 0.00025, has_chop);
            if error <= tolerance { break; }
