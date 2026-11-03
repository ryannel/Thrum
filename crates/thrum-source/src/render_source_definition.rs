//! Immutable source meaning for the inactive shared render-v1 decoder.
//!
//! This descriptor is SDK-owned text, not editable source or an admission
//! certificate. Its exact bytes use the existing built-in definition domain.
//! Target evaluator revisions and representation certificates remain F4/V3
//! work; this file neither registers the schema nor selects a device profile.

/// Complete source grammar, including exact predicates and material meaning.
/// `?` means absent or present with the stated type; explicit null refuses.
/// Identity-keyed collection order is immaterial except for diagnostics.
pub(crate) const RENDER_DEFINITION: &str = concat!(
    "thrum-builtin-definition-v1\nthrum.render.scene@1\n",
    "envelope=id:uuid,type:thrum.render.scene,schema:1,fields:object,documentation?:utf8<=4096\n",
    "uuid=decoded-string;exactly-36-ascii-bytes;lowercase-hex;hyphens-at-8,13,18,23;version-nibble=1..8;variant-nibble=8|9|a|b\n",
    "fields=cameras:camera[],active_camera:required-same-root-ref<camera>,meshes:mesh[],instances:instance[],materials:material[],textures:texture[],local_lights:point-light[];all-seven-required\n",
    "objects=maps-only;unknown=refuse;duplicate-decoded-key=refuse;optional=absent-or-typed;null=refuse;extensions=none\n",
    "json=strict-finite-i-json;decode-string-escapes-before-key-enum-uuid-validation;record-bytes<=16777216;value-nodes-including-containers<=1000000;container-depths=0..63;root-depth=0;fields-depth=1\n",
    "numbers=pinned-finite-binary64-decoder;decoded-f64-values-are-meaning;subnormals-preserved;semantic-negative-zero=positive-zero;source-revision=exact-file-bytes\n",
    "integer-tokens=unsigned-json-integer-token-only;decimal-or-exponent-spelling=refuse;u32=0..4294967295;u64=0..18446744073709551615\n",
    "identity=one-uuid-per-embedded-object;unique-in-complete-project-uuid-namespace-including-project-history-schema-top-level-and-embedded-identities;embedded-not-separate-file-or-manifest-record-identity\n",
    "collections=identity-keyed;semantic-product-enumeration=uuid-order;array-reorder-alone-preserves-meaning;ordered-payloads=mesh-positions,mesh-uv0,mesh-triangles,texture-bytes\n",
    "edges=required-typed-same-owning-render-root;active_camera->camera;instance.mesh->mesh;instance.material->material;material.base_texture->texture:rgba8-srgb;material.orm_texture->texture:rgba8-linear;material.emissive_texture->texture:rgba8-srgb\n",
    "edge-closure=all-declared-roots-including-unselected;present-optional-texture-edge-is-required-even-if-factor-zero;missing-or-wrong-type-or-cross-root=refuse;aggregate-required-edge-accounting-includes-embedded-edges\n",
    "validation-phases=whole-record-envelope,complete-local-structure,global-identity-uniqueness,typed-same-root-closure,deferred-instance-transformed-box;no-provisional-projection-is-admitted-source\n",
    "structural-collection-diagnostics=cameras,meshes,instances,materials,textures,local_lights;first-failing-collection-wins-independent-of-json-key-order\n",
    "collection-error-order=identified-before-anonymous;identified:uuid,field-order,raw-object-bytes,source-ordinal;anonymous:source-ordinal,field-order\n",
    "object-diagnostics=recognized-decoded-key-duplicates-before-values-in-schema-field-order;unknown-last;unknown-key-lexicographically-smallest;sampler-error-occupies-material-sampler-field\n",
    "camera-fields=id:uuid,position_m:finite-f64[3],orientation_xyzw:quaternion,vertical_fov_rad:finite-f64,near_m:finite-f64,far_m:finite-f64,exposure:finite-f64;all-required;diagnostic-order=listed\n",
    "camera=perspective;right-handed-y-up;forward=-Z;position_m-authored-initial-world-metres;each-position-component-in-closed[-1000000,1000000];0<vertical_fov_rad<mathematical-pi;0<near_m<far_m;exposure>0-dimensionless;aspect=derived-current-viewport\n",
    "quaternion=finite-f64[4]-xyzw;Q=x*x+y*y+z*z+w*w-exact-dyadic;epsilon=exact-rational-1/1000000000;(1-epsilon)^2<=Q<=(1+epsilon)^2;rotation=R=N/Q\n",
    "quaternion-N-rows=(w*w+x*x-y*y-z*z,2*(x*y-w*z),2*(x*z+w*y));(2*(x*y+w*z),w*w-x*x+y*y-z*z,2*(y*z-w*x));(2*(x*z-w*y),2*(y*z+w*x),w*w-x*x-y*y+z*z);right-handed-column-vectors;sign-and-nonzero-scalar-equivalent-quaternions-mean-same-rotation\n",
    "mesh-fields=id:uuid,positions_m:finite-f64[3][],uv0:finite-f64[2][],triangles:u32[3][];all-required;diagnostic-order=listed\n",
    "mesh=ordered-local-metre-positions;exact-dyadic-squared-norm-of-each-position<=1000000;uv0-count=position-count;uv0[i]-belongs-to-positions_m[i];uv-finite-without-range-clamp;uv-(0,0)=top-left-image-edge\n",
    "triangle=three-distinct-in-range-indices-into-own-mesh;exact-dyadic-cross(P1-P0,P2-P0)-has-nonzero-component;no-minimum-area;index-order-defines-oriented-winding;either-winding-valid\n",
    "mesh-admission-box=tight-closed-componentwise-extrema-of-all-positions-including-unreferenced;empty-positions-require-empty-uv0-and-triangles;empty-box=local-origin-point\n",
    "instance-fields=id:uuid,mesh:required-same-root-ref<mesh>,material:required-same-root-ref<material>,translation_m:finite-f64[3],orientation_xyzw:quaternion,scale:finite-f64[3];all-required;diagnostic-order=listed\n",
    "instance=authored-initial-TRS;affine-image=t+R(q)*diag(scale)*p;each-translation-component-in-closed[-1000000,1000000];finite-negative-and-zero-scale-valid-subject-to-bound;two_sided-policy-from-material\n",
    "instance-transformed-bound=entire-affine-image-of-local-admission-box;each-exact-rational-coordinate-bound-in-closed[-1000000,1000000];boundary-equality-admits;conservative-box-rule-includes-empty-corners;not-only-actual-vertices\n",
    "material-fields=id:uuid,alpha_mode:opaque|mask,two_sided:boolean,sampler:sampler,alpha_cutoff?:unit-f64,base_factor_rgba?:unit-f64[4],emissive_radiance_factor_cd_m2?:nonnegative-f64[3],metallic_factor?:unit-f64,roughness_factor?:unit-f64,occlusion_strength?:unit-f64,base_texture?:required-same-root-ref<texture>,orm_texture?:required-same-root-ref<texture>,emissive_texture?:required-same-root-ref<texture>;first-four-required;diagnostic-order=listed\n",
    "unit-f64=finite-binary64-in-closed[0,1];nonnegative-f64=finite-binary64>=0;material-cutoff=required-for-mask-forbidden-for-opaque\n",
    "material-factor-defaults=base_factor_rgba:[1,1,1,1];emissive_radiance_factor_cd_m2:[0,0,0];metallic_factor:0;roughness_factor:1;occlusion_strength:1\n",
    "material-absent-slot-defaults=base:[1,1,1,1];orm:[1,1,1];emissive:[1,1,1];normal:flat;normal-scale:1;normal-map-and-smooth-normal-fields=unknown-refusal\n",
    "material-working-basis=linear-rec709;emissive-factor-units=cd/m2;base-and-alpha-factors=dimensionless;all-texture-slots-share-uv0-and-one-sampler\n",
    "material-effective-equations=baseRGBA=base_factor_rgba*baseTexelRGBA;emissionRGB=emissive_radiance_factor_cd_m2*emissiveTexelRGB;metalness=metallic_factor*ORM.b;perceptualRoughness=max(0.045,roughness_factor*ORM.g);alphaGGX=perceptualRoughness^2;AO=1+occlusion_strength*(ORM.r-1);products=componentwise\n",
    "material-AO=selected-indirect-light-only-including-authored-ambient-Lambert;no-direct-light-emission-visibility-or-background-modulation;roughness-floor-after-factor-texture-multiply-before-square\n",
    "material-coverage=opaque-always-covered;mask-covered-iff-baseRGBA.a>=alpha_cutoff;equality-covered;depth-shadow-forward-share-sampled-alpha-factor-cutoff-and-UV\n",
    "material-closure=single-layer-single-scatter-metallic-roughness;Lambert-diffuse+GGX-height-correlated-Smith-Schlick-specular;F0=(1-metalness)*[0.04,0.04,0.04]+metalness*baseRGBA.rgb;finite-precision-evaluator-versioned-by-product\n",
    "material-BRDF=ci=n.dot(wi),co=n.dot(wo),h=normalize(wi+wo),ch=n.dot(h),m=metalness,base=baseRGBA.rgb,a=alphaGGX;ci<=0-or-co<=0:zero;otherwise-fr=(1-m)*(1-F)*base/pi+D*G2*F/(4*ci*co);F=F0+(1-F0)*(1-wo.dot(h))^5;D=a^2/(pi*(1+ch^2*(a^2-1))^2);G2=2*ci*co/(ci*sqrt(a^2+(1-a^2)*co^2)+co*sqrt(a^2+(1-a^2)*ci^2))\n",
    "material-two_sided=authored-policy;false:front-face-only;true:both-faces;both-boolean-values-source-valid;target-back-face-shading-implementation-selected-and-versioned-later\n",
    "sampler-fields=min_filter:nearest|linear,mag_filter:nearest|linear,address_u:clamp-to-edge|repeat|mirror-repeat,address_v:clamp-to-edge|repeat|mirror-repeat;all-required;diagnostic-order=listed;recognized-duplicates-before-values;unknown-last;one-level-no-mip-choice\n",
    "sampler-coordinates=normalized-edge;top-left-edge=(0,0);texel-centre(i,j)=((i+1/2)/width,(j+1/2)/height);sRGB-RGB-decodes-to-linear-before-filtering;alpha-and-data-filter-in-stored-linear-unit-interval;straight-unpremultiplied-RGBA-channels-filter-independently\n",
    "sampler-ideal-address(k,N)=clamp-to-edge:min(max(k,0),N-1);repeat:r=k-mod-N-in-[0,N),result=r;mirror-repeat:r=k-mod-2N-in-[0,2N),result=(r<N?r:2N-1-r)\n",
    "sampler-ideal-nearest=unaddressed-i=floor(width*u),j=floor(height*v);then-address-each-integer-index;exact-nearest-tie-chooses-higher-unaddressed-index\n",
    "sampler-ideal-linear=x=width*u-1/2,y=height*v-1/2;i=floor(x),j=floor(y);s=x-i,t=y-j;address-each-of-four-integer-taps-independently;result=(1-t)*((1-s)*T(i,j)+s*T(i+1,j))+t*((1-s)*T(i,j+1)+s*T(i+1,j+1))\n",
    "sampler-ideal-decode=C=byte/255;rgba8-srgb-RGB=(C<=0.04045?C/12.92:((C+0.055)/1.055)^2.4);rgba8-srgb-alpha=C;rgba8-linear-all-channels=C;decode-before-filter;ideal-formulas-over-exact-reals\n",
    "sampler-target=ideal-law-is-source-meaning;finite-precision-evaluation-including-nearest-tie-implementation-filter-selection-and-LOD-selected-and-versioned-later;equal-min-mag-not-required-by-source;source-does-not-promise-bit-identical-GPU-samples\n",
    "texture-fields=id:uuid,width:positive-u64,height:positive-u64,format:rgba8-srgb|rgba8-linear,rgba8:canonical-padded-rfc4648-base64-string;all-required;diagnostic-order=listed\n",
    "texture=exactly-4*width*height-decoded-bytes;upper-left-origin-row-major-RGBA;no-row-padding;no-mips;checked-dimension-and-encoded-length-arithmetic-before-allocation;decoded-length-and-canonical-padding-bits-checked\n",
    "point-light-fields=id:uuid,position_m:finite-f64[3],luminous_intensity_cd:nonnegative-f64,unit_luminance_linear_rec709:nonnegative-f64[3],radius_m:nonnegative-f64;all-required;diagnostic-order=listed\n",
    "point-light=authored-initial-world-position-metres;each-position-component-in-closed[-1000000,1000000];raw-intensity-candela;radius-metres-has-no-source-maximum;zero-signed-zero-subnormal-values-retain-decoded-meaning\n",
    "point-light-unit-luminance=Y=(1063*r+3576*g+361*b)/5000;exact-rationals-over-decoded-f64;abs(Y-1)<=1/1000000000;boundary-equality-admits;preserve-decoded-colour;product-normalization-and-residual-versioned-later\n",
    "point-light-transport=inverse-square-outside-declared-emitter-radius;inside-radius-target-support-is-F4/V3-work;does-not-change-constant-light-source-predicate\n",
    "runtime-authority=W1-committed-camera-instance-light-poses;source-poses=authored-initial-values-only;projection-and-exposure-remain-authored;viewport-aspect-and-view-origin-derived;origin-shift-does-not-change-physical-source-signs\n",
    "limits=no-schema-specific-camera-mesh-instance-material-texture-light-count-cap;common-source-capacities-and-required-edge-accounting-still-apply;host-identity-work-pressure=retryable-incomplete-validation-not-new-source-meaning\n",
    "support-boundary=source-validity-independent-of-recipe-capacity-target-representation-and-device-support;F4-may-refuse-valid-source-before-ready-publication;source-schema-hash-is-not-product-evaluator-key\n",
);

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const DOMAIN: &[u8] = b"thrum-f3-builtin-schema-v1\0";

    fn hash(domain: &[u8], descriptor: &str) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(domain);
        digest.update(descriptor.as_bytes());
        digest.finalize().into()
    }

    #[test]
    fn exact_render_descriptor_bytes_have_a_frozen_domain_separated_hash() {
        assert_eq!(
            hash(DOMAIN, RENDER_DEFINITION),
            [
                175, 247, 67, 221, 183, 192, 250, 136, 53, 239, 164, 23, 229, 27, 156, 8, 61, 118,
                1, 26, 146, 173, 211, 61, 248, 142, 84, 157, 50, 127, 60, 240
            ]
        );
        assert_ne!(
            hash(b"thrum-f3-type-version-v1\0", RENDER_DEFINITION),
            hash(DOMAIN, RENDER_DEFINITION)
        );
    }

    #[test]
    fn selected_source_meaning_mutations_change_the_definition_hash() {
        let original = hash(DOMAIN, RENDER_DEFINITION);
        for (from, to) in [
            (
                "required-same-root-ref<camera>",
                "required-project-ref<camera>",
            ),
            ("exact-dyadic-cross", "rounded-f64-cross"),
            ("base_factor_rgba:[1,1,1,1]", "base_factor_rgba:[0,0,0,1]"),
            ("metallic_factor:0", "metallic_factor:1"),
            ("baseRGBA.a>=alpha_cutoff", "baseRGBA.a>alpha_cutoff"),
            ("abs(Y-1)<=1/1000000000", "abs(Y-1)<1/1000000000"),
            (
                "each-exact-rational-coordinate-bound",
                "each-rounded-f64-coordinate-bound",
            ),
            (
                "sRGB-RGB-decodes-to-linear-before-filtering",
                "sRGB-RGB-decodes-after-filtering",
            ),
            (
                "exact-nearest-tie-chooses-higher-unaddressed-index",
                "exact-nearest-tie-chooses-lower-unaddressed-index",
            ),
        ] {
            assert!(RENDER_DEFINITION.contains(from), "missing mutation: {from}");
            let changed = RENDER_DEFINITION.replacen(from, to, 1);
            assert_ne!(hash(DOMAIN, &changed), original, "mutation: {from}");
        }
    }

    #[test]
    fn render_definition_does_not_activate_or_replace_existing_builtins() {
        use crate::environment_json::{ENVIRONMENT_DEFINITION, LIGHT_DEFINITION, SCENE_DEFINITION};
        use crate::registry_stage::builtin_schema;

        assert!(builtin_schema("thrum.render.scene", 1).is_none());
        assert!(builtin_schema("core.scene", 3).is_none());
        for (record_type, schema, descriptor, expected_hash) in [
            (
                "core.scene",
                2,
                SCENE_DEFINITION,
                "75c09153b459c576f89ad3328dc6e3bac82379368cd7e3ac9317f058a2ea8aa7",
            ),
            (
                "thrum.environment.authored",
                1,
                ENVIRONMENT_DEFINITION,
                "8f3d060dcd716a40ebfeddcdae58cc7285d281b6bba24fa9a38ebf2000ab5cfb",
            ),
            (
                "thrum.light.identity",
                1,
                LIGHT_DEFINITION,
                "9376fbe1d51794e5cae447c41ae0d02a71cd3f652830c9b650375144ac97b166",
            ),
        ] {
            let active = builtin_schema(record_type, schema).unwrap();
            assert_eq!(active.descriptor(), descriptor);
            assert_eq!(active.definition_hash(), &hash(DOMAIN, descriptor));
            let actual_hex = active
                .definition_hash()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(actual_hex, expected_hash);
            assert_ne!(active.descriptor(), RENDER_DEFINITION);
        }
    }
}
