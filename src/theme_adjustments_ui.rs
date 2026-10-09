fn theme_adjustment_key(is_3d: bool, set: PieceSet, theme: crate::board3d::Theme) -> String {
    if is_3d { format!("3d:{theme:?}") } else { format!("2d:{}",set.label()) }
}

// Original Ironwood appearance controls; no additional assets or dependencies.
impl ChessApp {
    fn sync_theme_adjustments(&mut self) {
        let key = theme_adjustment_key(self.board_3d_active,self.piece_set,self.board_3d_theme);
        if key == self.adjustment_theme_key { return; }
        self.theme_adjustments.insert(self.adjustment_theme_key.clone(),self.render_piece_appearance());
        let settings = self.theme_adjustments.get(&key).copied().unwrap_or_default().normalized();
        self.piece_appearance=settings;
        self.piece_shadows=settings.shadows;
        if self.board_3d_active {
            self.board_3d_appearance=settings.board_appearance.unwrap_or(85);
            self.show_radial_light=settings.radial_light.unwrap_or(true);
            self.board_3d_appearance_customized=true;
        }
        self.adjustment_theme_key=key;
        self.board_3d_gpu_key.clear(); self.board_3d_render_key.clear();
        self.save_preferences();
    }

    fn render_piece_appearance(&self) -> crate::board3d::PieceAppearance {
        crate::board3d::PieceAppearance { outline: !self.board_3d_active && self.piece_appearance.outline, shadows: self.piece_shadows, board_appearance:Some(self.board_3d_appearance), radial_light:Some(self.show_radial_light), ..self.piece_appearance.normalized() }
    }

    fn theme_adjustments_dialog(&mut self, ctx: &egui::Context) {
        if !self.theme_adjustments_open { return; }
        self.sync_theme_adjustments();
        let before = self.render_piece_appearance();
        let mut open = true;
        let label = if self.board_3d_active { crate::board3d::Theme::ALL.iter().find(|(theme,_)| *theme == self.board_3d_theme).map_or("3D",|(_,label)| *label) } else { self.piece_set.label() };
        egui::Window::new(format!("Theme adjustments - {label}"))
            .id(egui::Id::new("theme_adjustments_window"))
            .open(&mut open).resizable(false).default_width(500.0)
            .frame(Self::dialog_frame()).show(ctx, |ui| {
                ui.label(RichText::new("Changes appear on the board immediately.").small().weak());
                ui.add_space(10.0);
                egui::ScrollArea::vertical().max_height((ctx.screen_rect().height()-160.0).max(200.0)).show(ui, |ui| {
                    if self.board_3d_active { self.theme_adjustments_3d_ui(ui); } else { self.theme_adjustments_2d_ui(ui); }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);
                    if ui.button("Reset this theme").clicked() { self.piece_appearance=Default::default(); self.piece_shadows=true; if self.board_3d_active { self.board_3d_appearance=85; self.board_3d_appearance_customized=true; self.show_radial_light=true; } }
                });
            });
        self.theme_adjustments_open = open;
        if before != self.render_piece_appearance() {
            self.board_3d_gpu_key.clear(); self.board_3d_render_key.clear();
            self.theme_adjustments.insert(self.adjustment_theme_key.clone(),self.render_piece_appearance());
            self.save_preferences(); ctx.request_repaint();
        }
    }

    fn theme_adjustments_2d_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing=Vec2::new(12.0,10.0);
        adjustment_section(ui,"Piece proportions",|ui| {
            egui::Grid::new("2d_size").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                adjustment_slider(ui,"Size",&mut self.piece_appearance.size,65..=115,true);
            });
        });
        adjustment_section(ui,"Piece colors",|ui| { self.piece_colors_ui(ui); });
        adjustment_section(ui,"Gradient colors",|ui| {
            ui.add_enabled_ui(self.piece_set != PieceSet::System,|ui| {
                ui.checkbox(&mut self.piece_appearance.gradient,"Use gradient colors");
                ui.add_enabled_ui(self.piece_appearance.gradient,|ui| {
                    egui::Grid::new("gradient_colors").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                        ui.label("White pieces"); ui.horizontal(|ui| { ui.label("Start"); ui.color_edit_button_srgb(&mut self.piece_appearance.white); ui.label("End"); ui.color_edit_button_srgb(&mut self.piece_appearance.white_end); }); ui.end_row();
                        ui.label("Black pieces"); ui.horizontal(|ui| { ui.label("Start"); ui.color_edit_button_srgb(&mut self.piece_appearance.black); ui.label("End"); ui.color_edit_button_srgb(&mut self.piece_appearance.black_end); }); ui.end_row();
                        ui.label("Direction");
                        let directions=["Top to bottom","Left to right","Diagonal","Center outward"];
                        egui::ComboBox::from_id_salt("gradient_direction").width(210.0).selected_text(directions[self.piece_appearance.gradient_direction as usize]).show_ui(ui,|ui| {
                            for (i,label) in directions.iter().enumerate() { ui.selectable_value(&mut self.piece_appearance.gradient_direction,i as u8,*label); }
                        }); ui.end_row();
                    });
                    ui.horizontal_wrapped(|ui| {
                        for (label,start,end,dark_start,dark_end) in [("Warm ivory",[255,244,203],[174,126,64],[109,118,132],[22,29,43]),("Ice & plum",[215,250,255],[69,155,203],[198,124,202],[65,30,87]),("Copper & teal",[255,204,155],[164,74,37],[112,218,196],[18,74,80])] {
                            if ui.button(label).clicked() { self.piece_appearance.white=start; self.piece_appearance.white_end=end; self.piece_appearance.black=dark_start; self.piece_appearance.black_end=dark_end; }
                        }
                    });
                });
            });
            ui.label(RichText::new("Gradients replace solid side colors and retain the artwork’s shading.").small().weak());
        });
        adjustment_section(ui,"Depth & outlines",|ui| {
            ui.add_enabled_ui(self.piece_set != PieceSet::System,|ui| {
                egui::Grid::new("2d_shading").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                    adjustment_slider(ui,"Inner shading",&mut self.piece_appearance.inner_shading,0..=100,true);
                });
            });
            ui.label(RichText::new(if self.piece_set == PieceSet::System { "Inner shading is available for illustrated piece sets." } else { "Light from the upper left adds volume inside each piece." }).small().weak());
            ui.checkbox(&mut self.piece_appearance.outline,"Contrasting piece outlines");
        });
        adjustment_section(ui,"Piece shadows",|ui| {
            ui.checkbox(&mut self.piece_shadows,"Enable shadows");
            ui.add_enabled_ui(self.piece_shadows,|ui| {
                egui::Grid::new("2d_shadow_rows").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                    adjustment_slider(ui,"Strength",&mut self.piece_appearance.shadow_strength,0..=150,true);
                    adjustment_slider(ui,"Softness",&mut self.piece_appearance.shadow_softness,0..=100,false);
                });
            });
        });
    }

    fn piece_colors_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.piece_appearance.custom_colors,"Use custom side colors");
        ui.add_enabled_ui(self.piece_appearance.custom_colors,|ui| {
            ui.columns(2,|columns| {
                columns[0].horizontal(|ui| { ui.label("White pieces"); ui.color_edit_button_srgb(&mut self.piece_appearance.white); });
                columns[1].horizontal(|ui| { ui.label("Black pieces"); ui.color_edit_button_srgb(&mut self.piece_appearance.black); });
            });
            ui.horizontal_wrapped(|ui| {
                for (label,white,black) in [("Ivory & charcoal",[235,232,225],[62,73,68]),("Gold & silver",[224,181,79],[145,159,174]),("Ice & plum",[151,211,229],[150,50,110])] {
                    if ui.button(label).clicked() { self.piece_appearance.white=white; self.piece_appearance.black=black; }
                }
            });
        });
    }

    fn theme_adjustments_3d_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(12.0,10.0);
        adjustment_section(ui,"Piece proportions",|ui| {
            egui::Grid::new("piece_proportion_rows").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                adjustment_slider(ui,"Size",&mut self.piece_appearance.size,65..=115,true);
                adjustment_slider(ui,"Height",&mut self.piece_appearance.height,75..=125,true);
            });
        });
        adjustment_section(ui,"Colors & material",|ui| {
            self.piece_colors_ui(ui);
            egui::Grid::new("piece_finish_row").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                ui.label("Material finish");
                let finishes=["Theme default","Matte","Satin","Glossy","Metallic","Glass"];
                egui::ComboBox::from_id_salt("piece_material_finish").width(210.0).selected_text(finishes[self.piece_appearance.finish as usize]).show_ui(ui,|ui| {
                    for (index,label) in finishes.iter().enumerate() { ui.selectable_value(&mut self.piece_appearance.finish,index as u8,*label); }
                }); ui.end_row();
            });

        });
        adjustment_section(ui,"Piece shadows",|ui| {
            ui.checkbox(&mut self.piece_shadows,"Enable shadows");
            ui.add_enabled_ui(self.piece_shadows,|ui| {
                egui::Grid::new("piece_shadow_rows").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                    adjustment_slider(ui,"Strength",&mut self.piece_appearance.shadow_strength,0..=150,true);
                    adjustment_slider(ui,"Softness",&mut self.piece_appearance.shadow_softness,0..=100,false);
                });
            });
        });
        adjustment_section(ui,"Board lighting",|ui| {
            egui::Grid::new("board_lighting_rows").min_col_width(145.0).spacing(Vec2::new(16.0,10.0)).show(ui,|ui| {
                let before=self.board_3d_appearance;
                adjustment_slider(ui,"3D appearance",&mut self.board_3d_appearance,0..=100,false);
                if before != self.board_3d_appearance { self.board_3d_appearance_customized=true; }
            });
            ui.checkbox(&mut self.show_radial_light,"Radial light below board");
        });
    }

    fn paint_adjusted_piece(&self, ui: &egui::Ui, rect: egui::Rect, cell: f32, side: Color, piece: Piece) {
        let a = self.piece_appearance.normalized();
        let rect = egui::Rect::from_center_size(rect.center(),rect.size()*a.size as f32/100.0);
        let cell = cell*a.size as f32/100.0;
        let color = if side == Color::White { a.white } else { a.black };
        if !a.custom_colors && !a.gradient && !a.outline && a.inner_shading==0 { Self::paint_piece(ui,rect,cell,self.piece_set,side,piece); return; }
        if let Some(source) = Self::piece_image_for(self.piece_set,side,piece) {
            let uri = source.uri().unwrap_or("").to_owned();
            let raster_width = 384; // Fixed resolution bounds processing and stabilizes the cache.
            let cache_settings = crate::board3d::PieceAppearance { custom_colors:a.custom_colors, white:a.white, black:a.black, outline:a.outline, inner_shading:a.inner_shading, gradient:a.gradient, gradient_direction:a.gradient_direction, white_end:a.white_end, black_end:a.black_end, ..Default::default() };
            let id = egui::Id::new(("adjusted_piece_v3",uri.as_str()));
            type Cache = (crate::board3d::PieceAppearance,u32,egui::TextureHandle);
            let cached = ui.ctx().data(|d| d.get_temp::<Cache>(id));
            let texture = if let Some((settings,_,texture)) = cached.filter(|(settings,width,_)| *settings == cache_settings && *width == raster_width) { let _ = settings; Some(texture) } else {
                let _ = source.clone().load(ui.ctx(),egui::TextureOptions::LINEAR,egui::load::SizeHint::Width(raster_width));
                if let Ok(egui::load::ImagePoll::Ready { image }) = ui.ctx().try_load_image(&uri,egui::load::SizeHint::Width(raster_width)) {
                    let image = adjusted_piece_pixels(&image,a,side);
                    let texture = ui.ctx().load_texture(format!("piece_look_{uri}"),image,egui::TextureOptions::LINEAR);
                    ui.ctx().data_mut(|d| d.insert_temp(id,(cache_settings,raster_width,texture.clone()))); Some(texture)
                } else { None }
            };
            if let Some(texture) = texture { egui::Image::new(&texture).maintain_aspect_ratio(true).paint_at(ui,rect.shrink(cell*0.06)); }
            else { Self::paint_piece(ui,rect,cell,self.piece_set,side,piece); ui.ctx().request_repaint(); }
        } else {
            let glyph = Self::piece_glyph_for(side,piece);
            let tint = if a.custom_colors { Color32::from_rgb(color[0],color[1],color[2]) } else if side == Color::White { Color32::from_rgb(250,246,224) } else { Color32::from_rgb(24,29,32) };
            let font = FontId::proportional(cell*0.74);
            if a.outline {
                let outline = if tint.r() as u16+tint.g() as u16+tint.b() as u16 > 380 { Color32::BLACK } else { Color32::WHITE };
                for (x,y) in [(-1.5,0.0),(1.5,0.0),(0.0,-1.5),(0.0,1.5)] { ui.painter().text(rect.center()+Vec2::new(x,y),Align2::CENTER_CENTER,glyph,font.clone(),outline); }
            }
            ui.painter().text(rect.center(),Align2::CENTER_CENTER,glyph,font,tint);
        }
    }
}

fn adjusted_piece_pixels(image: &egui::ColorImage, a: crate::board3d::PieceAppearance, side: Color) -> egui::ColorImage {
    // Run only on a texture-cache miss, with bounded 384px source images.
    // Premultiplied filtering avoids dark fringes at transparent edges.
    let smoothed = if a.outline {
        let bytes: Vec<u8> = image.pixels.iter().flat_map(|pixel| pixel.to_array()).collect();
        let rgba = image::RgbaImage::from_raw(image.size[0] as u32,image.size[1] as u32,bytes).unwrap();
        let filtered = image::imageops::blur(&rgba,(image.size[0] as f32/192.0*0.7).max(0.35));
        Some(egui::ColorImage::new(image.size,filtered.pixels().map(|p| Color32::from_rgba_premultiplied(p[0],p[1],p[2],p[3])).collect()))
    } else { None };
    let image = smoothed.as_ref().unwrap_or(image);
    let mut output = image.clone();
    let color = if side == Color::White { a.white } else { a.black };
    if a.custom_colors || a.gradient {
        let [width,height]=image.size;
        let end=if side==Color::White { a.white_end } else { a.black_end };
        for (index,pixel) in output.pixels.iter_mut().enumerate() {
            let color=if a.gradient { gradient_piece_color(color,end,a.gradient_direction,(index%width) as f32/(width.saturating_sub(1)).max(1) as f32,(index/width) as f32/(height.saturating_sub(1)).max(1) as f32) } else { color };
            let [r,g,b,alpha] = pixel.to_srgba_unmultiplied();
            let shade = 0.3 + 0.7*(r as f32*0.2126+g as f32*0.7152+b as f32*0.0722)/255.0;
            *pixel = Color32::from_rgba_unmultiplied((color[0] as f32*shade) as u8,(color[1] as f32*shade) as u8,(color[2] as f32*shade) as u8,alpha);
        }
    }
    if a.inner_shading>0 {
        let [width,height]=image.size;
        let amount=a.inner_shading as f32/100.0;
        // One scan per row derives a rounded cross-section from the silhouette.
        for y in 0..height {
            let row=&image.pixels[y*width..(y+1)*width];
            let Some(left)=row.iter().position(|p| p.a()>128) else { continue; };
            let right=row.iter().rposition(|p| p.a()>128).unwrap();
            if right<=left { continue; }
            for x in left..=right {
                let index=y*width+x;
                let [r,g,b,alpha]=output.pixels[index].to_srgba_unmultiplied();
                if alpha==0 { continue; }
                let nx=2.0*(x-left) as f32/(right-left) as f32-1.0;
                let rounded=(1.0-nx*nx).max(0.0).sqrt();
                let light=(0.85*rounded-0.45*nx).max(0.0);
                let factor=1.0+amount*(light*0.55-0.4);
                let highlight=amount*light.powi(6)*12.0;
                let channel=|c:u8| (c as f32*factor+highlight*(c as f32/255.0).sqrt()).clamp(0.0,255.0) as u8;
                output.pixels[index]=Color32::from_rgba_unmultiplied(channel(r),channel(g),channel(b),alpha);
            }
        }
    }
    if a.outline {
        let [width,height] = image.size;
        let light = if a.custom_colors || a.gradient { color.iter().map(|c| *c as u16).sum::<u16>() > 380 } else { side == Color::White };
        let outline = if light { Color32::from_rgb(15,18,22) } else { Color32::from_rgb(245,243,229) };
        // Circular dilation with fractional coverage keeps curves smooth. Composite
        // behind the source instead of replacing its antialiased edge pixels.
        let radius = (width as f32 / 192.0 * 4.0).max(0.75);
        let feather = (width as f32 / 192.0 * 1.25).max(0.75);
        // Two distance sweeps replace the per-pixel neighborhood search.
        // Work stays linear in image size, independent of outline thickness.
        let mut distance: Vec<f32> = image.pixels.iter().map(|p| if p.a()>0 { 1.0-p.a() as f32/255.0 } else { 1.0e6 }).collect();
        for y in 0..height { for x in 0..width {
            let i=y*width+x;
            if x>0 { distance[i]=distance[i].min(distance[i-1]+1.0); }
            if y>0 {
                distance[i]=distance[i].min(distance[i-width]+1.0);
                if x>0 { distance[i]=distance[i].min(distance[i-width-1]+std::f32::consts::SQRT_2); }
                if x+1<width { distance[i]=distance[i].min(distance[i-width+1]+std::f32::consts::SQRT_2); }
            }
        }}
        for y in (0..height).rev() { for x in (0..width).rev() {
            let i=y*width+x;
            if x+1<width { distance[i]=distance[i].min(distance[i+1]+1.0); }
            if y+1<height {
                distance[i]=distance[i].min(distance[i+width]+1.0);
                if x>0 { distance[i]=distance[i].min(distance[i+width-1]+std::f32::consts::SQRT_2); }
                if x+1<width { distance[i]=distance[i].min(distance[i+width+1]+std::f32::consts::SQRT_2); }
            }
        }}
        for y in 0..height { for x in 0..width {
            let index=y*width+x;
            let source=output.pixels[index];
            if source.a()==255 { continue; }
            let coverage=((radius+feather-distance[index])/(2.0*feather)).clamp(0.0,1.0);
            let behind=coverage*0.9*(1.0-source.a() as f32/255.0);
            let channel=|src:u8,edge:u8| (src as f32+edge as f32*behind).round().min(255.0) as u8;
            output.pixels[index]=Color32::from_rgba_premultiplied(channel(source.r(),outline.r()),channel(source.g(),outline.g()),channel(source.b(),outline.b()),(source.a() as f32+255.0*behind).round().min(255.0) as u8);
        }}
    }
    output
}

fn gradient_piece_color(start:[u8;3],end:[u8;3],direction:u8,x:f32,y:f32) -> [u8;3] {
    let t=match direction { 1=>x,2=>(x+y)*0.5,3=>((x-0.5).powi(2)+(y-0.5).powi(2)).sqrt()*2.0,_=>y }.clamp(0.0,1.0);
    std::array::from_fn(|i| (start[i] as f32*(1.0-t)+end[i] as f32*t).round() as u8)
}

fn adjustment_section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(Color32::from_rgb(25,30,34)).corner_radius(6.0)
        .inner_margin(14.0).show(ui,|ui| {
            ui.set_min_width((ui.available_width()).max(200.0));
            ui.label(RichText::new(title).strong().color(Color32::from_rgb(211,173,98)));
            ui.add_space(3.0);
            content(ui);
        });
    ui.add_space(2.0);
}

fn adjustment_slider(ui: &mut egui::Ui, label: &str, value: &mut u8, range: std::ops::RangeInclusive<u8>, percent: bool) {
    ui.label(label);
    ui.spacing_mut().slider_width=200.0;
    let slider=egui::Slider::new(value,range);
    ui.add(if percent { slider.suffix("%") } else { slider });
    ui.end_row();
}

#[cfg(test)]
mod outline_tests {
    use super::*;
    #[test]
    fn gradients_interpolate_and_preserve_transparency() {
        assert_eq!(gradient_piece_color([255;3],[0;3],0,0.5,0.0),[255;3]);
        assert_eq!(gradient_piece_color([255;3],[0;3],1,1.0,0.0),[0;3]);
        assert_eq!(gradient_piece_color([255;3],[0;3],3,0.5,0.5),[255;3]);
        let image=egui::ColorImage::new([4,4],vec![Color32::from_rgba_unmultiplied(255,255,255,128);16]);
        let settings=crate::board3d::PieceAppearance { gradient:true,white:[255,0,0],white_end:[0,0,255],..Default::default() };
        let output=adjusted_piece_pixels(&image,settings,Color::White);
        assert!(output.pixels[0].r()>output.pixels[0].b());
        assert!(output.pixels[15].b()>output.pixels[15].r());
        assert!(output.pixels.iter().all(|p| p.a()==128));
        let json=serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<crate::board3d::PieceAppearance>(&json).unwrap(),settings);
    }
    #[test]
    fn inner_shading_preserves_alpha_and_adds_directional_depth() {
        let mut image=egui::ColorImage::new([32,32],vec![Color32::TRANSPARENT;1024]);
        for y in 4..28 { for x in 4..28 { image.pixels[y*32+x]=Color32::from_rgb(150,120,90); }}
        let settings=crate::board3d::PieceAppearance { inner_shading:100,..Default::default() };
        let shaded=adjusted_piece_pixels(&image,settings,Color::White);
        assert_eq!(adjusted_piece_pixels(&image,Default::default(),Color::White),image);
        assert!(shaded.pixels[16*32+10].r()>shaded.pixels[16*32+21].r());
        assert_ne!(shaded,image);
        for (before,after) in image.pixels.iter().zip(&shaded.pixels) { assert_eq!(before.a(),after.a()); }
    }
    #[test]
    fn smooth_outline_preserves_source_and_has_symmetric_fractional_edges() {
        let mut image=egui::ColorImage::new([32,32],vec![Color32::TRANSPARENT;1024]);
        for y in 10..22 { for x in 10..22 { image.pixels[y*32+x]=Color32::from_rgb(40,30,20); }}
        let settings=crate::board3d::PieceAppearance { outline:true,..Default::default() };
        let output=adjusted_piece_pixels(&image,settings,Color::Black);
        assert_eq!(output.pixels[16*32+16],image.pixels[16*32+16]);
        assert_eq!(output.pixels[0],Color32::TRANSPARENT);
        assert!(output.pixels[16*32+9].a()>0 && output.pixels[16*32+9].a()<255);
        for y in 0..32 { for x in 0..32 { assert_eq!(output.pixels[y*32+x],output.pixels[y*32+31-x]); }}
    }
}
