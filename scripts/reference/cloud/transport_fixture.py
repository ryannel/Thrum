"""Retained independent Beer/slab fixtures for CLOUDS transport_gpu.

Run in the reference environment with NumPy 2.5.5. The oracle is imported
unchanged; neither the GPU shader nor the cloud model supplies expected values.
Output is whitespace-separated so Cargo tests need no Python or JSON dependency.
"""
import optical_reference as reference


def main():
    print('# sigma source length coarse_step fine_step T S composed')
    for sigma, length, source in [(0., 800., .001), (1e-8, 800., .001),
                                  (.001, 800., .001), (.01, 800., .002),
                                  (.05, 80., .005)]:
        transmission, scatter = reference.beer_segment(sigma, source, length)
        # Known physical source: unit beam distributed isotropically, with no
        # self-shadow. This last slab cannot scatter more energy than enters.
        if sigma > 0:
            assert scatter <= source / sigma
        background, air_t, air_s = .2, .7, .04
        composed = background * transmission + air_t * scatter + (1-transmission) * air_s
        print(*(format(x, '.17g') for x in
                [sigma, source, length, length/8, length/128,
                 transmission, scatter, composed]))


if __name__ == '__main__':
    main()
