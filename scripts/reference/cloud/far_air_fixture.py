"""Independent finite air-path reference used by far_aerial_gpu.

Run with NumPy 2.5.5. The authored RGB atmosphere of air_reference is retained
unchanged; the engine's spectral calibration is deliberately not substituted.
The resulting discrepancy is measured, not called pure LUT interpolation error.
"""
import math
import air_reference as air
import numpy as np

optical=air.optical


def finite_scatter(direction, distance):
    """Reference's independent Gaussian source integral stopped at the endpoint."""
    origin=np.array([0.,260.4846459239257,0.])
    direction=np.array(direction)
    distance=min(distance,optical.trace_terminal(origin,direction)['distance_m'])
    nodes,weights=optical._gauss(256)
    t=distance*(nodes+1)/2
    positions=origin+t[:,None]*direction
    tau_view=optical._tau_many(np.broadcast_to(origin,positions.shape),direction,t,256,optical.EARTH)
    sun=np.array([0.,1.,0.])
    paths=[optical.trace_terminal(p,sun)['distance_m'] for p in positions]
    tau_sun=optical._tau_many(positions,sun,paths,256,optical.EARTH)
    _,rayleigh,aerosol=optical.coefficients(optical.radial_height(positions),optical.EARTH)
    source=np.exp(-tau_sun)*100000*(rayleigh*optical.phase_rayleigh(direction[1])+aerosol*optical.phase_hg(direction[1],optical.EARTH.aerosol_g))
    return distance/2*np.sum(weights[:,None]*np.exp(-tau_view)*source,axis=0)

print('# elevation_degrees distance_m T_RGB direct_S_RGB; observer 260.4846459239257 m')
for elevation in [0., 1., 5., 15., 60.]:
    angle = math.radians(elevation)
    direction = [math.cos(angle), math.sin(angle), 0.]
    for distance in [1000., 31999., 32000., 32001., 40000., 150000., 200000., 1e7]:
        height=260.4846459239257
        length=min(distance,float(air.distance_to_top(height,direction[1])))
        endpoint=np.array([0.,height,0.])+np.array(direction)*length
        end_height=min(float(optical.radial_height(endpoint)),air.ATMOSPHERE.top_m)
        end_mu=((air.ATMOSPHERE.radius_m+height)*direction[1]+length)/(air.ATMOSPHERE.radius_m+end_height)
        transmission=air.oracle_top(height,direction[1],order=512)/air.oracle_top(end_height,end_mu,order=512)
        print(elevation, distance, *transmission,*finite_scatter(direction,distance))
