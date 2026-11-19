//! Inspector editing for a scene-authored NPC destination.
use crate::{App, component_fields::morph_edit_token};
use voxy_gameplay::{CharacterBody, NpcDestination};

pub(crate) const SCHEMA: &str = "game.npc-destination.v1";

pub(crate) fn field_label(path: &str) -> &str {
    match path {
        "/goal/0" => "Destination X",
        "/goal/1" => "Destination Z",
        "/origin/0" => "Region origin X",
        "/origin/1" => "Region origin Z",
        "/cells/0" => "Cells along X",
        "/cells/1" => "Cells along Z",
        "/cell_size" => "Cell size (m)",
        "/floor_y/0" => "Lowest floor Y",
        "/floor_y/1" => "Highest floor Y",
        "/max_step_height" => "Max step (m)",
        "/max_slope_tan" => "Max slope tangent",
        "/min_clearance" => "Headroom (m)",
        _ => path,
    }
}

impl App {
    pub(crate) fn toggle_npc_destination(
        &mut self,
        token: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get_mut(self.selected)
            .ok_or("missing NPC owner")?;
        if morph_edit_token(object) != token {
            return Err("NPC changed before activation".into());
        }
        if object.components.remove(SCHEMA).is_none() {
            let owner = self.resolve_object(&object.id).ok_or("missing NPC owner")?;
            let body = self
                .scene
                .component::<CharacterBody>(owner)?
                .ok_or("add a character before setting an NPC destination")?;
            if self
                .scene
                .component::<voxy_gameplay::NpcRoute>(owner)?
                .is_some()
            {
                return Err("remove the authored NPC route before adding a destination".into());
            }
            let position = self.scene.world_matrix(owner)?.w_axis.truncate().as_dvec3();
            let [half_x, half_y, half_z] = body.half_extents;
            let cell_size = (2. * half_x.max(half_z)).max(0.25);
            let offset = 8.5 * f64::from(cell_size);
            let floor = position.y - f64::from(half_y);
            let destination = NpcDestination {
                origin: [position.x - offset, position.z - offset],
                cells: [16; 2],
                cell_size,
                floor_y: [floor - 0.25, floor + 0.25],
                max_step_height: 0.02,
                max_slope_tan: 0.6,
                min_clearance: 2. * half_y,
                goal: [position.x, position.z],
            };
            object
                .components
                .insert(SCHEMA.into(), serde_json::to_value(destination)?);
        }
        let page = crate::component_fields::fields(object)?
            .iter()
            .position(|field| field.schema == SCHEMA && field.path == "/goal/0")
            .map_or(0, |index| index / 6);
        self.commit_follow_document(document)?;
        self.inspector = crate::InspectorMode::Components(page);
        Ok(())
    }
}
