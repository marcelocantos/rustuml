// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import net.sourceforge.plantuml.*;
import net.sourceforge.plantuml.core.TextBlockExporter12026;
import net.sourceforge.plantuml.sequencediagram.*;
import net.sourceforge.plantuml.sequencediagram.teoz.*;
import net.sourceforge.plantuml.skin.rose.Rose;
public class HistoryPaintProbe {
 static Object field(Object object,String name)throws Exception{Field f=object.getClass().getDeclaredField(name);f.setAccessible(true);return f.get(object);}
 static class Trace extends LiveBoxes {
  final List<Event> events;
  Trace(Participant p,List<Event> events,SequenceDiagram diagram){super(p,events,new Rose(),diagram.getSkinParam());this.events=events;}
  public void addStep(Event event,double y){super.addStep(event,y);if(event.dealWith(getParticipant()))System.out.println("CALL "+getParticipant().getCode()+" "+events.indexOf(event)+" "+y);}
  public Stairs getStairs(double createY,double height){var stairs=super.getStairs(createY,height);System.out.println("PAINT "+getParticipant().getCode()+" "+height);for(var step:stairs.getSteps())System.out.println("STAIR "+step.getValue()+" "+step.getIndent());return stairs;}
 }
 public static void main(String[]args)throws Exception{
  var diagram=(SequenceDiagram)new SourceStringReader(Files.readString(Path.of(args[0]))).getBlocks().get(0).getDiagram();
  var format=new FileFormatOption(FileFormat.SVG);var maker=new SequenceDiagramFileMakerTeoz(diagram,new Rose(),format,0);
  var events=new ArrayList<Event>();for(Event e:diagram.events())events.add(e);
  var spaces=(LivingSpaces)field(maker,"livingSpaces");
  for(var space:spaces.values()){var f=LivingSpace.class.getDeclaredField("liveboxes");f.setAccessible(true);var old=(LiveBoxes)f.get(space);f.set(space,new Trace(old.getParticipant(),events,diagram));}
  try(var output=Files.newOutputStream(Path.of(args[1]))){TextBlockExporter12026.builder(maker.getTextBlock12026(0,format),format,false).styled(diagram).build().exportTo(output);}
 }
}
