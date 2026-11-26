    let dx=dpdx(input.rest);let dy=dpdy(input.rest);
    let wake_slope=wake_at(input.rest,0.0).yz;
    let wake_dx=dpdx(wake_slope);let wake_dy=dpdy(wake_slope);
