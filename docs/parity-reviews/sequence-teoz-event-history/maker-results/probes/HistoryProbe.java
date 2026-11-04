// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import net.sourceforge.plantuml.*;
import net.sourceforge.plantuml.sequencediagram.*;
import net.sourceforge.plantuml.sequencediagram.teoz.*;
import net.sourceforge.plantuml.skin.rose.Rose;
import net.sourceforge.plantuml.klimt.drawing.LimitFinder;
public class HistoryProbe {
 static Object field(Object object, String name) throws Exception {
  Field f=object.getClass().getDeclaredField(name); f.setAccessible(true); return f.get(object);
 }
 public static void main(String[] args) throws Exception {
  var diagram=(SequenceDiagram)new SourceStringReader(Files.readString(Path.of(args[0]))).getBlocks().get(0).getDiagram();
  var format=new FileFormatOption(FileFormat.SVG);
  var maker=new SequenceDiagramFileMakerTeoz(diagram,new Rose(),format,0);
  var bounder=format.getDefaultStringBounder(diagram.getSkinParam());
  var spaces=(LivingSpaces)field(maker,"livingSpaces");
  var events=new ArrayList<Event>(); for(Event e:diagram.events())events.add(e);
  var block=maker.getTextBlock12026(0,format);
  for(int phase=0;phase<3;phase++) {
   if(phase==0)block.calculateDimension(bounder); else block.drawU(LimitFinder.create(bounder,true));
   System.out.println("PHASE "+phase);
   for(var space:spaces.values()) {
    var boxes=(LiveBoxes)field(space,"liveboxes");
    System.out.println("PARTICIPANT "+boxes.getParticipant().getCode());
    var steps=(Map<?,?>)field(boxes,"eventsStep");
    for(int i=0;i<events.size();i++)System.out.println(i+" "+events.get(i).getClass().getSimpleName()+" callback="+steps.get(events.get(i))+" level="+boxes.getLevelAt(events.get(i),EventsHistoryMode.CONSIDERE_FUTURE_DEACTIVATE));
    for(var step:boxes.getStairs(0,1000).getSteps())System.out.println("STAIR "+step.getValue()+" "+step.getIndent());
   }
  }
 }
}
