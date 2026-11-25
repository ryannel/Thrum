            weight+=prepared.w;
        }
    }
    radiances[lane]=radiance;
    weights[lane]=weight;
    workgroupBarrier();
    // Disjoint four-lane reductions. Lane zero normalizes once at the end.
    if local_id.x<2u {
        radiances[lane]+=radiances[lane+2u];
        weights[lane]+=weights[lane+2u];
    }
    workgroupBarrier();
    if inside && local_id.x==0u {
        let sum=radiances[lane]+radiances[lane+1u];
        let sum_weight=weights[lane]+weights[lane+1u];
        textureStore(reflection_output,vec2<i32>(id.xy),i32(id.z),sum/sum_weight);
    }
}
